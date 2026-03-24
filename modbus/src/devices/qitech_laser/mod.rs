use crate::{Device, HandleResponseError, Priority, Request, Response, Scheduler};

#[derive(Debug, Clone)]
pub struct LaserDevice<S: Scheduler> {
    scheduler: S,

    // data
    measurement: Option<Measurement>,
}

impl<S: Scheduler> LaserDevice<S> {
    pub fn measurement(&self) -> Option<&Measurement> {
        self.measurement.as_ref()
    }

    pub fn refresh_measurement(&mut self) {
        self.scheduler.schedule(Priority::Low);
    }
}

impl<S: Scheduler> Device<S> for LaserDevice<S> {
    fn new(scheduler: S) -> Self
    where
        Self: Sized,
    {
        Self {
            scheduler,
            measurement: None,
        }
    }

    fn next_request(&mut self) -> Option<(Request, bool)> {
        Some((Request::ReadInputRegisters(0x0E, 3), false))
    }

    fn handle_response(&mut self, result: Response) -> Result<(), HandleResponseError> {
        use HandleResponseError::InvalidFunctionCode;

        let words = match result {
            Response::ReadInputRegisters(v) => v,
            rsp => return Err(InvalidFunctionCode(rsp.function_code())),
        };

        debug_assert!(words.len() == 3);

        self.measurement = Some(Measurement {
            diameter: words[0],
            x_axis: words[1],
            y_axis: words[2],
        });

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub diameter: u16,
    pub x_axis: u16,
    pub y_axis: u16,
}
