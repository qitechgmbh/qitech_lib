
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Telemetry {
    pub status: Status,
    pub sensors: Sensors,
    pub error: ErrorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    DataTooSmall(u32,u16),
    InvalidValue(u32,u16),
}

impl Telemetry {
    pub fn from_vec(words: Vec<u16>) -> Result<Self, ParseError> {
        if words.len() < 6 {
            let err = ParseError::DataTooSmall(6, words.len() as u16);
            return Err(err);
        }

        let status = match Status::try_from(words[3]) {
            Ok(v) => v,
            Err(v) => return Err(ParseError::InvalidValue(3, v)),
        };

        let error = match ErrorCode::try_from(words[4]) {
            Ok(v) => v,
            Err(v) => return Err(ParseError::InvalidValue(4, v)),
        };

        let sensors = Sensors {
            voltage: words[0],
            current: words[1],
            temperature: words[2],
            frequency: words[5],
        };

        Ok(Self {
            status,
            sensors,
            error,
        })
    }
}

// status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Running,
    Fault,
}

impl TryFrom<u16> for Status {
    type Error = u16;

    fn try_from(value: u16) -> Result<Self, u16> {
        match value {
            0 => Ok(Status::Idle),
            1 => Ok(Status::Running),
            2 => Ok(Status::Fault),
            v => Err(v),
        }
    }
}

// sensors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sensors {
    /// (0 - 990) -> (0 - 99hz)
    pub frequency: u16,

    /// (0 - 990) -> (0 - 99hz)
    pub voltage: u16,

    /// (0 - 990) -> (0 - 99hz)
    pub current: u16,

    /// (0 - 990) -> (0 - 99hz)
    pub temperature: u16,
}

// Error Codes

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    None,                       // 0
    PulseOvercurrent,           // 1
    IgbtOvercurrentProtection,  // 2
    DcBusOvervoltageProtection, // 3
    TemperatureNearIgbtLimit,   // 4
    InverterThermalProtection,  // 5
    InverterOverload100Percent, // 6
    InverterPowerCut,           // 7
}

impl TryFrom<u16> for ErrorCode {
    type Error = u16;

    fn try_from(value: u16) -> Result<Self, u16> {
        match value {
            0 => Ok(ErrorCode::None),
            1 => Ok(ErrorCode::PulseOvercurrent),
            2 => Ok(ErrorCode::IgbtOvercurrentProtection),
            3 => Ok(ErrorCode::DcBusOvervoltageProtection),
            4 => Ok(ErrorCode::TemperatureNearIgbtLimit),
            5 => Ok(ErrorCode::InverterThermalProtection),
            6 => Ok(ErrorCode::InverterOverload100Percent),
            7 => Ok(ErrorCode::InverterPowerCut),
            v => Err(v),
        }
    }
}

// MotationState
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionState {
    Stop,
    Forward,
    Reverse,
}

impl TryFrom<u16> for MotionState {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, ()> {
        use MotionState::*;

        match value {
            1 => Ok(Forward),
            2 => Ok(Reverse),
            3 => Ok(Stop),
            _ => Err(()),
        }
    }
}

impl From<MotionState> for u16 {
    fn from(value: MotionState) -> Self {
        use MotionState::*;

        match value {
            Forward => 1,
            Reverse => 2,
            Stop => 3,
        }
    }
}

// Config
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub motion_state: MotionState,
    pub frequency_target: u16,
    pub acceleration_level: u16,
    pub deceleration_level: u16,
}

impl Config {
    pub fn from_vec(words: Vec<u16>) -> Result<Self, ParseError> {
        if words.len() < 4 {
            let err = ParseError::DataTooSmall(4, words.len() as u16);
            return Err(err);
        }

        let motion_state = match MotionState::try_from(words[0]) {
            Ok(v) => v,
            Err(_) => return Err(ParseError::InvalidValue(0, words[0])),
        };

        let frequency_target = match words[1] {
            x @ 0..=9900 => x,
            x => return Err(ParseError::InvalidValue(1, x)),
        };

        let acceleration_level = match words[2] {
            x @ 1..=15 => x,
            x => return Err(ParseError::InvalidValue(2, x)),
        };

        let deceleration_level = match words[3] {
            x @ 1..=15 => x,
            x => return Err(ParseError::InvalidValue(3, x)),
        };

        Ok(Self {
            motion_state,
            frequency_target,
            acceleration_level,
            deceleration_level,
        })
    }

    pub fn to_words(&self) -> [u16; 4] {
        [
            self.motion_state.clone().into(),
            self.frequency_target.clone(),
            self.acceleration_level.clone(),
            self.deceleration_level.clone(),
        ]
    }

    pub fn with_mutation(&self, m: &ConfigMutation) -> Self {
        Self {
            motion_state: m.motion_state.clone().unwrap_or(self.motion_state.clone()),
            frequency_target: m.frequency_target.unwrap_or(self.frequency_target),
            acceleration_level: m.acceleration_level.unwrap_or(self.acceleration_level),
            deceleration_level: m.deceleration_level.unwrap_or(self.deceleration_level),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigMutation {
    pub motion_state: Option<MotionState>,
    pub frequency_target: Option<u16>,
    pub acceleration_level: Option<u16>,
    pub deceleration_level: Option<u16>,
}
