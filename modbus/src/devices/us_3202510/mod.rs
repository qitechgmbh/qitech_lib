mod types;

pub use types::{Config, ConfigMutation, MotionState, Status, Telemetry};

use crate::{
    ModbusDevice, ModbusRequest, ModbusResponse, ModbusSettings, ModbusType, Parity,
    SerialDeviceMeta, create_modbus_device_context,
};
use common::get_async_runtime;
use std::{borrow::Cow, fmt, time::Duration};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use tokio_modbus::{
    ExceptionCode, Request, Response,
    client::{Client, Context},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceRequest {
    RefreshTelemetry,
    SyncConfig,
    MutateConfig,
}

struct ActorMessage {
    request: ModbusRequest,
    reply_tx: oneshot::Sender<Result<ModbusResponse, VfdError>>,
}

struct PendingRequest {
    request_type: DeviceRequest,
    rx: oneshot::Receiver<Result<ModbusResponse, VfdError>>,
}

pub struct VfdDevice {
    config: Option<Config>,
    telemetry: Option<Telemetry>,

    config_mutation: ConfigMutation,

    scheduled_requests_buf: [(Priority, DeviceRequest); 3],
    scheduled_requests_len: usize,

    tx: mpsc::Sender<ActorMessage>,
    pending_request: Option<PendingRequest>,
    handle: JoinHandle<()>,
}

impl Drop for VfdDevice {
    fn drop(&mut self) {
        println!("VfdDevice drop is called");
        self.handle.abort();
    }
}

impl VfdDevice {
    // Getters

    pub fn telemetry(&self) -> Option<&Telemetry> {
        self.telemetry.as_ref()
    }

    pub fn actual_config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    pub fn projected_config(&self) -> Option<Config> {
        self.config.as_ref().map(|v| v.with_mutation(&self.config_mutation))
    }

    // Setters / Actions

    pub fn sync_config(&mut self) {
        self.schedule_request(Priority::Low, DeviceRequest::SyncConfig);
    }

    pub fn refresh_telemetry(&mut self) {
        self.schedule_request(Priority::Low, DeviceRequest::RefreshTelemetry);
    }

    pub fn stop_immediately(&mut self) {
        self.config_mutation.motion_state = Some(MotionState::Stop);
        self.schedule_request(Priority::Medium, DeviceRequest::MutateConfig);
    }

    pub fn set_motion_state(&mut self, value: MotionState) {
        self.config_mutation.motion_state = Some(value);
        self.schedule_request(Priority::Medium, DeviceRequest::MutateConfig);
    }

    pub fn set_frequency(&mut self, value: u16) {
        self.config_mutation.frequency_target = Some(value);
        self.schedule_request(Priority::Medium, DeviceRequest::MutateConfig);
    }

    pub fn set_acceleration_level(&mut self, value: u16) {
        self.config_mutation.acceleration_level = Some(value);
        self.schedule_request(Priority::Medium, DeviceRequest::MutateConfig);
    }

    pub fn set_deceleration_level(&mut self, value: u16) {
        self.config_mutation.deceleration_level = Some(value);
        self.schedule_request(Priority::Medium, DeviceRequest::MutateConfig);
    }

    // Queue management
    fn schedule_request(&mut self, priority: Priority, request: DeviceRequest) {
        for i in 0..self.scheduled_requests_len {
            if self.scheduled_requests_buf[i].1 == request {
                if priority > self.scheduled_requests_buf[i].0 {
                    self.scheduled_requests_buf[i].0 = priority;
                    self.resort();
                }
                return;
            }
        }

        if self.scheduled_requests_len < self.scheduled_requests_buf.len() {
            self.scheduled_requests_buf[self.scheduled_requests_len] = (priority, request);
            self.scheduled_requests_len += 1;
            self.resort();
        }
    }

    fn resort(&mut self) {
        for i in 1..self.scheduled_requests_len {
            let mut j = i;
            while j > 0 && self.scheduled_requests_buf[j].0 > self.scheduled_requests_buf[j - 1].0 {
                self.scheduled_requests_buf.swap(j, j - 1);
                j -= 1;
            }
        }
    }

    // Response Handlers

    fn handle_refresh_telemetry_words(&mut self, words: Vec<u16>) -> Result<(), anyhow::Error> {
        let telemetry = Telemetry::from_vec(words)
            .map_err(|e| anyhow::anyhow!("Failed to parse telemetry: {:?}", e))?;
        self.telemetry = Some(telemetry);
        Ok(())
    }

    fn handle_sync_config_words(&mut self, words: Vec<u16>) -> Result<(), anyhow::Error> {
        let config = Config::from_vec(words)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {:?}", e))?;
        self.config = Some(config);
        Ok(())
    }
}

impl ModbusDevice for VfdDevice {
    fn new(
        path: String,
        slave_id: u8,
        settings: Option<ModbusSettings>,
    ) -> Result<Self, anyhow::Error> {
        let meta = match settings {
            Some(s) => SerialDeviceMeta {
                path,
                device_name: None,
                slave_id,
                baudrate: s.baudrate,
                bits: s.bits,
                stop_bits: s.stop_bits,
                parity: s.parity,
                modbus_type: s.modbus_type,
            },
            None => SerialDeviceMeta {
                path,
                device_name: None,
                slave_id,
                baudrate: 9600,
                bits: 8,
                stop_bits: 1,
                parity: Parity::None,
                modbus_type: ModbusType::Rtu,
            },
        };

        let rt = get_async_runtime();
        let _g = rt.enter();

        let ctx = create_modbus_device_context(&meta)?;
        let (tx, rx) = mpsc::channel::<ActorMessage>(1);
        let handle = rt.spawn(run_modbus_actor(rx, ctx));

        let dummy_slot = (Priority::Low, DeviceRequest::MutateConfig);

        Ok(Self {
            config: None,
            telemetry: None,
            config_mutation: ConfigMutation::default(),
            scheduled_requests_buf: [dummy_slot.clone(), dummy_slot.clone(), dummy_slot],
            scheduled_requests_len: 0,
            tx,
            pending_request: None,
            handle,
        })
    }

    fn send_next_request(&mut self) -> Result<(), anyhow::Error> {
        if self.pending_response_in_flight() || self.scheduled_requests_len == 0 {
            return Ok(());
        }

        let (_, request_type) = self.scheduled_requests_buf[0].clone();

        let modbus_req = match request_type {
            DeviceRequest::RefreshTelemetry => Request::ReadInputRegisters(0x8, 6),
            DeviceRequest::SyncConfig => Request::ReadHoldingRegisters(0x2, 4),
            DeviceRequest::MutateConfig => {
                let data = match self.projected_config() {
                    Some(v) => Cow::Owned(v.to_words().to_vec()),
                    None => return Ok(()),
                };
                Request::WriteMultipleRegisters(0x2, data)
            }
        };

        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = ActorMessage {
            request: modbus_req,
            reply_tx,
        };

        match self.tx.try_send(msg) {
            Ok(_) => {
                // Remove scheduled request from front of buffer after sending
                for i in 0..self.scheduled_requests_len - 1 {
                    self.scheduled_requests_buf[i] = self.scheduled_requests_buf[i + 1].clone();
                }
                self.scheduled_requests_len -= 1;

                self.pending_request = Some(PendingRequest {
                    request_type,
                    rx: reply_rx,
                });
            }
            Err(mpsc::error::TrySendError::Full(_)) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }

        Ok(())
    }

    fn handle_response(&mut self) -> Result<(), anyhow::Error> {
        let is_ready = match &mut self.pending_request {
            Some(pending) => match pending.rx.try_recv() {
                Ok(result) => Some((pending.request_type.clone(), result)),
                Err(oneshot::error::TryRecvError::Empty) => None,
                Err(oneshot::error::TryRecvError::Closed) => {
                    return Err(anyhow::anyhow!("Oneshot channel dropped without a response"));
                }
            },
            None => return Ok(()),
        };

        if let Some((req_type, actor_result)) = is_ready {
            self.pending_request = None;

            let response = actor_result?;

            match (req_type, response) {
                (DeviceRequest::RefreshTelemetry, Response::ReadInputRegisters(words)) => {
                    self.handle_refresh_telemetry_words(words)?;
                }
                (DeviceRequest::SyncConfig, Response::ReadHoldingRegisters(words)) => {
                    self.handle_sync_config_words(words)?;
                }
                (DeviceRequest::MutateConfig, Response::WriteMultipleRegisters(..)) => {
                    // Successfully written
                }
                (_, rsp) => {
                    return Err(anyhow::anyhow!(
                        "Invalid Function Code returned: {}",
                        rsp.function_code()
                    ));
                }
            }
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl VfdDevice {
    fn pending_response_in_flight(&self) -> bool {
        self.pending_request.is_some()
    }
}

#[derive(Debug)]
pub enum VfdError {
    ModbusError(tokio_modbus::Error),
    TaskDied,
    ModbusException(ExceptionCode),
    IoErr,
    RequestTimeOut,
}

impl fmt::Display for VfdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfdError::ModbusError(e) => write!(f, "Modbus error: {}", e),
            VfdError::TaskDied => write!(f, "Internal driver task died"),
            VfdError::ModbusException(code) => write!(f, "Modbus exception: {:?}", code),
            VfdError::IoErr => write!(f, "Hardware I/O error"),
            VfdError::RequestTimeOut => write!(f, "Request timed out"),
        }
    }
}

impl std::error::Error for VfdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VfdError::ModbusError(e) => Some(e),
            VfdError::ModbusException(e) => Some(e),
            VfdError::TaskDied | VfdError::IoErr | VfdError::RequestTimeOut => None,
        }
    }
}

async fn run_modbus_actor(mut rx: mpsc::Receiver<ActorMessage>, mut ctx: Context) {
    let timeout_duration = Duration::from_secs(2);

    while let Some(msg) = rx.recv().await {
        println!("msg : {:?}",msg.request);
        let response_result = tokio::time::timeout(timeout_duration, ctx.call(msg.request)).await;
        println!("response_result : {:?}",response_result);

        let process_result = match response_result {
            Ok(Ok(Ok(response))) => Ok(response),
            Ok(Ok(Err(modbus_err))) => Err(VfdError::ModbusException(modbus_err)),
            Ok(Err(_io_err)) => Err(VfdError::IoErr),
            Err(_timeout_err) => Err(VfdError::RequestTimeOut),
        };
        let _ = msg.reply_tx.send(process_result);
    }

    let _ = ctx.disconnect().await;
    println!("VfdDevice background actor shut down cleanly.");
}