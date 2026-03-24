pub mod clients;
pub mod devices;
pub mod managers;

pub use tokio_modbus as protocol;

pub type Request = protocol::Request<'static>;
pub type Response = protocol::Response;
pub type ExceptionCode = protocol::ExceptionCode;
pub type FunctionCode = protocol::FunctionCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

pub trait Scheduler {
    /// notifies manager that this device
    /// want's to submit a request with
    /// certain priority.
    /// Operation should not fail.
    fn schedule(&self, priority: Priority);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// (index: u16, received: u16)
    InvalidValue(u16, u16),

    /// (minimum: u16, received: u16)
    DataTooSmall(u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleResponseError {
    ParseError(ParseError),
    InvalidFunctionCode(FunctionCode),
    NoResponseExpected,
}

pub trait Device<S: Scheduler> {
    /// creates a new device and assigns it it's scheduler
    /// to notify it's manager that it wants to submit
    /// a request
    fn new(scheduler: S) -> Self
    where
        Self: Sized;

    /// retrieve the devices next request if any.
    /// should only be called be the device manager
    /// if the device is scheduled
    fn next_request(&mut self) -> Option<(Request, bool)>;

    /// response forwarded by manager to device to handle
    fn handle_response(&mut self, response: Response) -> Result<(), HandleResponseError>;
}
