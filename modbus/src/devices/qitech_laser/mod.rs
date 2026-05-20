use common::get_async_runtime;
use std::{fmt, time::Duration};
use tokio::{sync::{mpsc, oneshot}, task::JoinHandle};
use tokio_modbus::{ExceptionCode, Request, Response, client::{Client, Context}};
use crate::{
    ModbusDevice, ModbusRequest, ModbusResponse, ModbusSettings, ModbusType, 
    Parity, SerialDeviceMeta, create_modbus_device_context
};

const MEASUREMENTS_ADDRESS: u16 = 0x0E;
const MEASUREMENTS_COUNT: u16 = 3;

// An internal envelope that pairs a request with its dedicated reply channel
struct ActorMessage {
    request: ModbusRequest,
    reply_tx: oneshot::Sender<Result<ModbusResponse, LaserError>>,
}

pub struct LaserDevice {
    pub measurement: Option<Measurement>,
    meta: SerialDeviceMeta,
    // The synchronous front-end holds the sender channel to talk to the actor
    tx: mpsc::Sender<ActorMessage>,
    // A oneshot receiver tracking a pending in-flight request
    pending_response: Option<oneshot::Receiver<Result<ModbusResponse, LaserError>>>,
    handle : JoinHandle<()>,
}

impl Drop for LaserDevice {
    fn drop(&mut self) {
        // Dropping the `tx` channel will cause the background actor's channel to close.
        // The background task will break out of its loop, run `ctx.disconnect().await`, 
        // and gracefully clean up the serial connection.
        println!("drop is called");
        self.handle.abort();
    }
}

impl ModbusDevice for LaserDevice {
    fn send_next_request(&mut self) -> Result<(), anyhow::Error> {
        // If we are already waiting for a response, don't spam another request
        if self.pending_response.is_some() {
            return Ok(());
        }

        let req = Request::ReadInputRegisters(MEASUREMENTS_ADDRESS, MEASUREMENTS_COUNT);
        let (reply_tx, reply_rx) = oneshot::channel();
        
        let msg = ActorMessage {
            request: req,
            reply_tx,
        };

        // Try sending to the background actor without blocking the synchronous loop
        match self.tx.try_send(msg) {
            Ok(_) => {
                self.pending_response = Some(reply_rx);
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
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

    fn new(path: String, slave_id: u8, settings: Option<ModbusSettings>) -> Result<Self, anyhow::Error> {
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
                baudrate: 38400,
                bits: 8,
                stop_bits: 1,
                parity: Parity::None,
                modbus_type: ModbusType::Rtu,
            },
        };
        let rt = get_async_runtime();
        let meta_clone = meta.clone();
        let g = rt.enter();
        // Initialize the Modbus Context initially
        let ctx = create_modbus_device_context(&meta)?;
        // Create an bounded command channel for our actor
        let (tx, rx) = mpsc::channel::<ActorMessage>(1);
        // Spawn the long-running async worker thread immediately
        let handle = rt.spawn(run_modbus_actor(rx, ctx, meta_clone));

        Ok(Self {
            measurement: None,
            meta,
            tx,
            pending_response: None,
            handle : handle,
        })
    }

    fn handle_response(&mut self) -> Result<(), anyhow::Error> {
        let is_ready = match &mut self.pending_response {
            Some(rx) => match rx.try_recv() {
                Ok(result) => Some(result),         // Response arrived!
                Err(oneshot::error::TryRecvError::Empty) => None, // Still processing
                Err(oneshot::error::TryRecvError::Closed) => {
                    return Err(anyhow::anyhow!("Oneshot channel dropped without a response"));
                }
            },
            None => return Ok(()), // No request currently pending
        };

        if let Some(actor_result) = is_ready {
            // Clear out the slot now that we consumed the response
            self.pending_response = None;

            let response = actor_result?; // Propagate errors thrown by the actor loop

            let words = match response {
                Response::ReadInputRegisters(v) => v,
                rsp => return Err(anyhow::anyhow!("Invalid Function Code: {}", rsp.function_code())),
            };

            debug_assert!(words.len() == 3);
            self.measurement = Some(Measurement {
                diameter: words[0],
                x_axis: words[1],
                y_axis: words[2],
            });
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum LaserError {
    ModbusError(tokio_modbus::Error),
    TaskDied(),
    ModbusException(ExceptionCode),
    IoErr(),
    RequestTimeOut,
}

impl fmt::Display for LaserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaserError::ModbusError(e) => write!(f, "Modbus error: {}", e),
            LaserError::TaskDied() => write!(f, "Internal driver task died"),
            LaserError::ModbusException(code) => write!(f, "Modbus exception: {:?}", code),
            LaserError::IoErr() => write!(f, "Hardware I/O error"),
            LaserError::RequestTimeOut => write!(f, "Request timed out"),
        }
    }
}

// 2. Implement the standard Error trait with your complete source routing
impl std::error::Error for LaserError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            // These sub-errors implement std::error::Error, so we return them
            LaserError::ModbusError(e) => Some(e),
            LaserError::ModbusException(e) => Some(e),
            
            // These errors do not have an underlying nested error payload
            LaserError::TaskDied() => None,
            LaserError::IoErr() => None,
            LaserError::RequestTimeOut => None,
        }
    }
}

/// The long-running asynchronous worker loop.
/// This completely owns and keeps the `Context` alive.
async fn run_modbus_actor(
    mut rx: mpsc::Receiver<ActorMessage>, 
    mut ctx: Context, 
    meta: SerialDeviceMeta
) {
    let timeout_duration = Duration::from_secs(2);

    // Loop until the LaserDevice front-end is dropped (closing the mpsc channel)
    while let Some(msg) = rx.recv().await {
        let response_result = tokio::time::timeout(timeout_duration, ctx.call(msg.request)).await;
        // Parse and send the final execution payload back across the oneshot wire
        let process_result = match response_result {
            Ok(Ok(Ok(response))) => Ok(response),
            Ok(Ok(Err(modbus_err))) => Err(LaserError::ModbusException(modbus_err)),
            Ok(Err(io_err)) => Err(LaserError::IoErr()),
            Err(_timeout_err) => Err(LaserError::RequestTimeOut),
        };
        let _ = msg.reply_tx.send(process_result);
    }

    // When the loop ends (LaserDevice dropped), cleanly disconnect the serial resource
    let _ = ctx.disconnect().await;
    println!("LaserDevice background actor shut down cleanly.");
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub diameter: u16,
    pub x_axis: u16,
    pub y_axis: u16,
}