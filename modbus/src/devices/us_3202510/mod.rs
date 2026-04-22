mod types;

pub use types::{Config, ConfigMutation, MotionState, Status, Telemetry};

use crate::{Device, HandleResponseError, Priority, Request, Response, Scheduler};

pub struct VfdDevice<S: Scheduler> {
    scheduler: S,

    /// values
    config: Option<Config>,
    telemetry: Option<Telemetry>,

    // request data
    config_mutation: ConfigMutation,

    /// stores queued requests
    scheduled_requests_buf: [(Priority, DeviceRequest); 3],
    scheduled_requests_len: usize,

    pending_request: Option<DeviceRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceRequest {
    RefreshTelemetry,
    SyncConfig,
    MutateConfig,
}

impl<S: Scheduler> VfdDevice<S> {
    // getters

    pub fn telemetry(&self) -> Option<&Telemetry> {
        self.telemetry.as_ref()
    }

    pub fn actual_config(&self) -> Option<&Config> {
        self.config.as_ref()
    }

    pub fn projected_config(&self) -> Option<Config> {
        match &self.config {
            Some(v) => Some(v.with_mutation(&self.config_mutation)),
            None => None,
        }
    }

    // setters

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

    // helpers

    fn schedule_request(&mut self, priority: Priority, request: DeviceRequest) {
        // Check if request already exists
        for i in 0..self.scheduled_requests_len {
            if self.scheduled_requests_buf[i].1 == request {
                // Same request type found
                if priority > self.scheduled_requests_buf[i].0 {
                    self.scheduled_requests_buf[i].0 = priority;
                    // Re-sort because priority changed
                    self.resort();
                }

                return;
            }
        }

        self.scheduled_requests_buf[self.scheduled_requests_len] = (priority, request);
        self.scheduled_requests_len += 1;
        self.resort();
        self.scheduler.schedule(priority);
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

    fn handle_refresh_telemetry(&mut self, response: Response) -> Result<(), HandleResponseError> {
        use HandleResponseError::{InvalidFunctionCode, ParseError};
        use tokio_modbus::Response::ReadInputRegisters;

        let words = match response {
            ReadInputRegisters(v) => v,
            rsp => return Err(InvalidFunctionCode(rsp.function_code())),
        };

        let telemetry = Telemetry::from_vec(words).map_err(ParseError)?;

        self.telemetry = Some(telemetry);

        Ok(())
    }

    fn handle_sync_config(&mut self, response: Response) -> Result<(), HandleResponseError> {
        use HandleResponseError::{InvalidFunctionCode, ParseError};
        use tokio_modbus::Response::ReadHoldingRegisters;

        let words = match response {
            ReadHoldingRegisters(v) => v,
            rsp => return Err(InvalidFunctionCode(rsp.function_code())),
        };

        let config = Config::from_vec(words).map_err(ParseError)?;

        self.config = Some(config);

        Ok(())
    }

    fn handle_mutate_config(&mut self, response: Response) -> Result<(), HandleResponseError> {
        use HandleResponseError::InvalidFunctionCode;
        use tokio_modbus::Response::WriteMultipleRegisters;

        match response {
            WriteMultipleRegisters(..) => Ok(()),
            rsp => Err(InvalidFunctionCode(rsp.function_code())),
        }
    }
}

impl<S: Scheduler + 'static> Device<S> for VfdDevice<S> {
    fn new(scheduler: S) -> Self
    where
        Self: Sized,
    {
        // just initialize with something
        let scheduled_requests_buf = [
            (Priority::Low, DeviceRequest::MutateConfig),
            (Priority::Low, DeviceRequest::MutateConfig),
            (Priority::Low, DeviceRequest::MutateConfig),
        ];

        Self {
            scheduler,
            config: None,
            telemetry: None,
            config_mutation: ConfigMutation::default(),
            scheduled_requests_buf,
            scheduled_requests_len: 0,
            pending_request: None,
        }
    }

    fn next_request(&mut self) -> Option<(Request, bool)> {
        use DeviceRequest::*;

        if self.scheduled_requests_len == 0 {
            return None;
        }

        let contains_more = self.scheduled_requests_len > 1;

        let (_, request) = &self.scheduled_requests_buf[0];

        self.pending_request = Some(request.clone());

        match request {
            RefreshTelemetry => Some((Request::ReadInputRegisters(0x8, 6), contains_more)),
            SyncConfig => Some((Request::ReadHoldingRegisters(0x2, 4), contains_more)),
            MutateConfig => {
                use std::borrow::Cow;

                let data = match self.projected_config() {
                    Some(v) => Cow::Owned(v.to_words().to_vec()),
                    None => return None,
                };

                Some((Request::WriteMultipleRegisters(0x2, data), contains_more))
            }
        }
    }

    fn handle_response(&mut self, response: Response) -> Result<(), HandleResponseError> {
        use DeviceRequest::*;
        use HandleResponseError::NoResponseExpected;

        let pending_request = match self.pending_request.clone() {
            Some(v) => v,
            None => return Err(NoResponseExpected),
        };

        match pending_request {
            RefreshTelemetry => self.handle_refresh_telemetry(response),
            SyncConfig => self.handle_sync_config(response),
            MutateConfig => self.handle_mutate_config(response),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
