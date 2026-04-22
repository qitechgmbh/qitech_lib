//! MinasA6B motor driver (CiA 402, PP-motion mode) for EtherCrab.
//!
//! PDO I/O
//!   - `input()`: unpack raw bytes from a `BitSlice` via `load_le` into a `TxPdo` struct.
//!   - `output()`: pack a `RxPdo` struct into raw bytes and write them via `store_le`.

use crate::coe::ConfigurableDevice;
use crate::devices::{
    EthercatDevice, EthercatDeviceProcessing, EthercatDeviceUsed, EthercatDynamicPDO, Module,
    NewEthercatDevice, SubDeviceIdentityTuple,
};
use crate::helpers::ethercrab_types::EthercrabSubDevicePreoperational;
use crate::helpers::minas_a6_subdevice_wrapper::{EtherCATSlaveWrapper, PdoMapping};
use crate::io::servo_velocity_minasa6::{MinasA6BDevice, MinasA6BInput, MinasA6BOutput};
use anyhow::{Error, anyhow};
use bitvec::field::BitField;
use bitvec::prelude::{BitSlice, Lsb0};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// Motor registers

pub struct Reg;

impl Reg {
    pub const EEPROM: u16 = 0x1010;
    pub const ENCODER_SETTING: u16 = 0x3015;
    pub const CONTROL_INPUT_SI_FUNCTION_ASSIGNMENT: u16 = 0x3400;
    pub const ENCODER_MULTI_TURN_DATA_CLEAR_TRIGGER: u16 = 0x4D00; // set from 0->1->0 after the register below has been set to 0x31
    pub const ENCODER_MULTI_TURN_DATA_CLEAR: u16 = 0x4D01; // set to 0x31 to clear the encoder data
    pub const SET_MODE_OF_OPERATION: u16 = 0x6060; // set tot 1 for pp mode
    pub const GET_MODE_OF_OPERATION: u16 = 0x6061;
    pub const POSITION_TARGET: u16 = 0x607A; // encoder-pulses
    pub const POSITION_ACTUAL: u16 = 0x6064; // encoder-pulses
    pub const TARGET_VELOCITY: u16 = 0x6081; // encoder-pulses/s
    pub const MAX_VELOCITY: u16 = 0x607F; // encoder-pulses/s
    pub const MAX_SPEED: u16 = 0x6080; // 1/min
    pub const TARGET_ACCELERATION: u16 = 0x6083; // encoder-pulses/s²
    pub const TARGET_DECELERATION: u16 = 0x6084; // encoder-pulses/s
    pub const ENCODER_RESOLUTION: u16 = 0x608F;
    pub const MAX_ACCELERATION: u16 = 0x60C5; // encoder-pulses/s

    pub const ERROR_CODE: u16 = 0x603F;
    pub const MAX_TORQUE: u16 = 0x6072; // in 0.1%

    pub const HOMING_MODE: u16 = 0x6098;
    pub const HOMING_SPEED: u16 = 0x6099; // first subindex: fast homing speed, second subindex: slow homing speed
    pub const HOMING_ACC: u16 = 0x609A;
    pub const HOMING_OFFSET: u16 = 0x607C;

    pub const FOLLOWING_WINDOW: u16 = 0x6065; // encoder-pulses
    pub const FOLLOWING_WINDOW_TIME: u16 = 0x6066; // ms
    pub const POSITION_WINDOW: u16 = 0x6067; // encoder-pulses
    pub const POSITION_WINDOW_TIME: u16 = 0x6068; // ms
    pub const POSITIONING_OPTION: u16 = 0x60F2;

    pub const CONTROL_WORD: u16 = 0x6040;
    pub const STATUS_WORD: u16 = 0x6041;

    pub const RX_PDO: u16 = 0x1600; // master -> slave
    pub const TX_PDO: u16 = 0x1A00; // slave -> master
    pub const RX_PDO_ASSIGN_ADDRESS: u16 = 0x1C12;
    pub const TX_PDO_ASSIGN_ADDRESS: u16 = 0x1C13;

    pub const CONTROL_WORD_DISABLE: u16 = 0x0000;
    pub const CONTROL_WORD_READY_TO_SWITCH_ON: u16 = 0x0006;
    pub const CONTROL_WORD_SWITCH_ON: u16 = 0x0007;
    pub const CONTROL_WORD_ENABLE: u16 = 0x000F;
}

impl Reg {
    // CiA 402 control words
    pub const CW_FAULT_RESET: u16 = 0x0080;
    pub const CW_QUICK_STOP: u16 = 0x0002;
}

pub struct MotorMode;
impl MotorMode {
    pub const PP: u8 = 1;
    pub const HOMING: u8 = 6;
}

// InputPdo  (slave -> master, TxPDO):  9 bytes
//   [0..1]  StatusWord  (u16 LE)
//   [2]     ModeDisplay (u8)
//   [3..4]  ErrorCode   (u16 LE)
//   [5..8]  PositionActual (i32 LE)
//
// OutputPdo (master -> slave, RxPDO): 19 bytes
//   [0..1]  ControlWord      (u16 LE)
//   [2]     Mode             (u8)
//   [3..6]  TargetPosition   (i32 LE)
//   [7..10] TargetVelocity   (u32 LE)
//   [11..14] TargetAccel     (u32 LE)
//   [15..18] TargetDecel     (u32 LE)

pub const TX_PDO_BYTES: usize = 9;
pub const RX_PDO_BYTES: usize = 19;

#[derive(Clone, Debug, Default)]
pub struct MinasA6BTxPdo {
    pub status_word: u16,
    pub mode_display: u8,
    pub error_code: u16,
    pub position_actual: i32,
}

impl MinasA6BTxPdo {
    pub fn from_bitslice(input: &BitSlice<u8, Lsb0>, bit_offset: usize) -> Self {
        let mut b = [0u8; TX_PDO_BYTES];
        for i in 0..TX_PDO_BYTES {
            b[i] = input[bit_offset + i * 8..bit_offset + (i + 1) * 8].load_le();
        }
        Self {
            status_word: u16::from_le_bytes([b[0], b[1]]),
            mode_display: b[2],
            error_code: u16::from_le_bytes([b[3], b[4]]),
            position_actual: i32::from_le_bytes([b[5], b[6], b[7], b[8]]),
        }
    }
}

/// Data sent to the drive each cycle (RxPDO: master -> slave).
#[derive(Clone, Debug, Default)]
pub struct MinasA6BRxPdo {
    pub control_word: u16,
    pub mode: u8,
    pub target_position: i32,
    pub target_velocity: u32,
    pub target_accel: u32,
    pub target_decel: u32,
}

impl MinasA6BRxPdo {
    pub fn to_bitslice(&self, output: &mut BitSlice<u8, Lsb0>, bit_offset: usize) {
        let cw = self.control_word.to_le_bytes();
        let tp = self.target_position.to_le_bytes();
        let tv = self.target_velocity.to_le_bytes();
        let ta = self.target_accel.to_le_bytes();
        let td = self.target_decel.to_le_bytes();

        let bytes: [u8; RX_PDO_BYTES] = [
            cw[0], cw[1], self.mode, tp[0], tp[1], tp[2], tp[3], tv[0], tv[1], tv[2], tv[3], ta[0],
            ta[1], ta[2], ta[3], td[0], td[1], td[2], td[3],
        ];

        for i in 0..RX_PDO_BYTES {
            output[bit_offset + i * 8..bit_offset + (i + 1) * 8].store_le(bytes[i]);
        }
    }
}

pub const SET_DATA_MAPPING: [PdoMapping; 6] = [
    PdoMapping {
        object_index: Reg::CONTROL_WORD,
        sub_index: 0,
        bit_length: 16,
    },
    PdoMapping {
        object_index: Reg::SET_MODE_OF_OPERATION,
        sub_index: 0,
        bit_length: 8,
    },
    PdoMapping {
        object_index: Reg::POSITION_TARGET,
        sub_index: 0,
        bit_length: 32,
    },
    PdoMapping {
        object_index: Reg::TARGET_VELOCITY,
        sub_index: 0,
        bit_length: 32,
    },
    PdoMapping {
        object_index: Reg::TARGET_ACCELERATION,
        sub_index: 0,
        bit_length: 32,
    },
    PdoMapping {
        object_index: Reg::TARGET_DECELERATION,
        sub_index: 0,
        bit_length: 32,
    },
];

pub const GET_DATA_MAPPING: [PdoMapping; 4] = [
    PdoMapping {
        object_index: Reg::STATUS_WORD,
        sub_index: 0,
        bit_length: 16,
    },
    PdoMapping {
        object_index: Reg::GET_MODE_OF_OPERATION,
        sub_index: 0,
        bit_length: 8,
    },
    PdoMapping {
        object_index: Reg::ERROR_CODE,
        sub_index: 0,
        bit_length: 16,
    },
    PdoMapping {
        object_index: Reg::POSITION_ACTUAL,
        sub_index: 0,
        bit_length: 32,
    },
];

// State machine

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Unknown,
    PreOpNotRdyToSwitchOn,
    PreOpSwitchOnDisabled,
    PreOpReadyToSwitchOn,
    Ready,
    RunIdle,
    RunSentMotionCommand,
    RunSentSetPointAck,
    RunWaitForSendingNextCommand,
    RunSentSetPointNack,
    RunExecuting,
    HomingIdle,
    HomingSendingHomeCommand,
    HomingWaitForFinish,
    HomingWaitForModeChange,
    QuickStopWaiting,
    QuickStopReached,
    ErrorAny,
    ErrorEncoder,
    ErrorGeneric,
    ErrorResetToggled,
}

impl State {
    pub fn is_error(self) -> bool {
        matches!(
            self,
            Self::ErrorAny | Self::ErrorEncoder | Self::ErrorGeneric | Self::ErrorResetToggled
        )
    }
    pub fn is_run(self) -> bool {
        matches!(
            self,
            Self::RunIdle
                | Self::RunSentMotionCommand
                | Self::RunSentSetPointAck
                | Self::RunWaitForSendingNextCommand
                | Self::RunSentSetPointNack
                | Self::RunExecuting
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomingDirection {
    /// Counter-clockwise: Panasonic homing mode 21.
    Ccw,
    /// Clockwise: Panasonic homing mode 19.
    Cw,
}

impl HomingDirection {
    pub fn mode_code(self) -> u8 {
        match self {
            HomingDirection::Ccw => 21,
            HomingDirection::Cw => 19,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MotorHomingConfig {
    pub homing_direction: HomingDirection,
    pub high_speed_rps: f64,
    pub slow_speed_rps: f64,
    pub acceleration_rps_squared: f64,
}

impl Default for MotorHomingConfig {
    fn default() -> Self {
        Self {
            homing_direction: HomingDirection::Ccw,
            high_speed_rps: 0.5,
            slow_speed_rps: 0.05,
            acceleration_rps_squared: 10.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EncoderResolution {
    pub increments: u32,
    pub revolutions: u32,
}

impl EncoderResolution {
    fn ratio(&self) -> f64 {
        self.increments as f64 / self.revolutions as f64
    }

    pub fn to_increments(&self, revolutions: f64, residue: &mut f64, is_absolute: bool) -> i32 {
        let f = if is_absolute {
            revolutions * self.ratio()
        } else {
            revolutions * self.ratio() + *residue
        };
        let i = f.floor() as i32;
        *residue = f - i as f64;
        i
    }

    pub fn to_revolutions(&self, increments: i32) -> f64 {
        increments as f64 / self.ratio()
    }

    pub fn rps_to_inc_per_sec(&self, rps: f64) -> u32 {
        (rps * self.ratio()) as u32
    }
}

#[derive(Clone, Debug)]
pub struct PositionSpec {
    pub n_revolutions: f64,
    pub is_absolute: bool,
    pub speed_rps: f64,
    pub acceleration_rps2: f64,
    pub deceleration_rps2: f64,
}

impl PositionSpec {
    pub fn new(
        n_revolutions: f64,
        is_absolute: bool,
        speed_rps: f64,
        acceleration_rps2: f64,
        deceleration_rps2: f64,
    ) -> Result<Self, Error> {
        if !n_revolutions.is_finite() {
            return Err(anyhow!("n_revolutions must be finite, got {n_revolutions}"));
        }
        for (name, v) in [
            ("speed_rps", speed_rps),
            ("acceleration_rps2", acceleration_rps2),
            ("deceleration_rps2", deceleration_rps2),
        ] {
            if !v.is_finite() || v <= 0.0 {
                return Err(anyhow!(
                    "{name} must be finite and > 0 (got {v}); control direction via the sign of n_revolutions"
                ));
            }
        }
        Ok(Self {
            n_revolutions,
            is_absolute,
            speed_rps,
            acceleration_rps2,
            deceleration_rps2,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PositionTransitionSpec {
    pub overlap: bool,
    pub delay_ms: f64,
}

// MinasA6BMotor

pub struct MinasA6BMotor {
    is_used: bool,
    tx_bit_offset: usize,
    rx_bit_offset: usize,
    pub module: Option<Module>,

    pub txpdo: MinasA6BTxPdo,
    pub rxpdo: MinasA6BRxPdo,

    // State machine
    pub state: State,

    // Encoder
    encoder_resolution: Option<EncoderResolution>,
    pulse_residue: f64,

    // Motion queue
    motion_out_deque: VecDeque<MinasA6BRxPdo>,
    motion_transition: Option<PositionTransitionSpec>,
    current_motion_command: Option<MinasA6BRxPdo>,
    previous_motion_command: Option<MinasA6BRxPdo>,
    movement_delay: Option<Instant>,

    // Mode tracking
    current_motor_mode: u8,

    // Control flags and timers
    single_enable_flag: bool,
    single_disable_flag: bool,
    last_fault_reset_toggle: Option<Instant>,
    mode_change_issued: Option<Instant>,
    homing_issued: Option<Instant>,
    pub needs_homing: bool,

    pub homing_config: MotorHomingConfig,
    pub initialized: bool,

    pub homed: bool,
}

impl Default for MinasA6BMotor {
    fn default() -> Self {
        Self {
            is_used: false,
            tx_bit_offset: 0,
            rx_bit_offset: 0,
            module: None,
            txpdo: MinasA6BTxPdo::default(),
            rxpdo: MinasA6BRxPdo::default(),
            state: State::Unknown,
            encoder_resolution: None,
            pulse_residue: 0.0,
            motion_out_deque: VecDeque::with_capacity(100),
            motion_transition: None,
            current_motion_command: None,
            previous_motion_command: None,
            movement_delay: None,
            current_motor_mode: MotorMode::PP,
            single_enable_flag: false,
            single_disable_flag: false,
            last_fault_reset_toggle: None,
            mode_change_issued: None,
            homing_issued: None,
            needs_homing: false,
            homing_config: MotorHomingConfig::default(),
            initialized: false,
            homed: false,
        }
    }
}

// Trait implementations

impl NewEthercatDevice for MinasA6BMotor {
    fn new() -> Self {
        Self {
            is_used: false,
            tx_bit_offset: 0,
            rx_bit_offset: 0,
            module: None,
            txpdo: MinasA6BTxPdo::default(),
            rxpdo: MinasA6BRxPdo::default(),
            state: State::Unknown,
            encoder_resolution: None,
            pulse_residue: 0.0,
            motion_out_deque: VecDeque::with_capacity(100),
            motion_transition: None,
            current_motion_command: None,
            previous_motion_command: None,
            movement_delay: None,
            current_motor_mode: MotorMode::PP,
            single_enable_flag: false,
            single_disable_flag: false,
            last_fault_reset_toggle: None,
            mode_change_issued: None,
            homing_issued: None,
            needs_homing: false,
            homing_config: MotorHomingConfig::default(),
            initialized: false,
            homed: false,
        }
    }
}

impl EthercatDeviceUsed for MinasA6BMotor {
    fn is_used(&self) -> bool {
        self.is_used
    }
    fn set_used(&mut self, used: bool) {
        self.is_used = used;
    }
}

impl EthercatDynamicPDO for MinasA6BMotor {
    fn get_tx_offset(&self) -> usize {
        self.tx_bit_offset
    }
    fn get_rx_offset(&self) -> usize {
        self.rx_bit_offset
    }
    fn set_tx_offset(&mut self, offset: usize) {
        self.tx_bit_offset = offset;
    }
    fn set_rx_offset(&mut self, offset: usize) {
        self.rx_bit_offset = offset;
    }
}

impl EthercatDeviceProcessing for MinasA6BMotor {}

impl EthercatDevice for MinasA6BMotor {
    fn into_any_boxed(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
    fn input(&mut self, input: &BitSlice<u8, Lsb0>) -> Result<(), Error> {
        self.txpdo = MinasA6BTxPdo::from_bitslice(input, self.tx_bit_offset);
        self.poll();
        Ok(())
    }

    fn input_len(&self) -> usize {
        TX_PDO_BYTES * 8
    }

    fn output(&self, output: &mut BitSlice<u8, Lsb0>) -> Result<(), Error> {
        self.rxpdo.to_bitslice(output, self.rx_bit_offset);
        Ok(())
    }

    fn output_len(&self) -> usize {
        RX_PDO_BYTES * 8
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn is_module(&self) -> bool {
        true
    }

    fn get_module(&self) -> Option<Module> {
        self.module.clone()
    }

    fn set_module(&mut self, module: Module) {
        self.tx_bit_offset = module.tx_offset;
        self.rx_bit_offset = module.rx_offset;
        self.module = Some(module);
    }

    fn input_checked(&mut self, input: &BitSlice<u8, Lsb0>) -> Result<(), Error> {
        let expected = self.input_len();
        let actual = input.len();
        if actual != expected {
            return Err(anyhow!(
                "[{}::input_checked] got {} bits ({} bytes), expected {} bits ({} bytes)",
                module_path!(),
                actual,
                actual / 8,
                expected,
                expected / 8
            ));
        }
        self.input(input)
    }

    fn output_checked(&self, output: &mut BitSlice<u8, Lsb0>) -> Result<(), Error> {
        let expected = self.output_len();
        let actual = output.len();
        if actual != expected {
            return Err(anyhow!(
                "[{}::output_checked] got {} bits ({} bytes), expected {} bits ({} bytes)",
                module_path!(),
                actual,
                actual / 8,
                expected,
                expected / 8
            ));
        }
        self.output(output)
    }
}

impl std::fmt::Debug for MinasA6BMotor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MinasA6BMotor(state={:?})", self.state)
    }
}

impl MinasA6BMotor {
    // This is for Debugging
    pub async fn read_digital_inputs<'a>(
        &self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<u32, Error> {
        let w = EtherCATSlaveWrapper::new(device);
        let di = w.read_sdo_u32(0x60FD, 0).await?;
        tracing::info!("Drive digital inputs 0x60FD: 0x{:08X}", di);
        tracing::info!(
            "  SI1={} SI2={} SI3={} SI4={} SI5={} SI6={} SI7={} SI8={}",
            (di >> 0) & 1, // SI1
            (di >> 1) & 1, // SI2
            (di >> 2) & 1, // SI3
            (di >> 3) & 1, // SI4
            (di >> 4) & 1, // SI5
            (di >> 5) & 1, // SI6
            (di >> 6) & 1, // SI7
            (di >> 7) & 1, // SI8
        );
        Ok(di)
    }
    pub async fn write_input_pin_functions<'a>(
        &self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<(), Error> {
        let w = EtherCATSlaveWrapper::new(device);
        let function_values: [u32; 8] = [
            0x00323232, // SI1
            0x00818181, // SI2
            0x00828282, // SI3
            0x00272727, // SI4
            0x00222222, // SI5
            0x00212121, // SI6
            0x00303030, // SI7
            0x00313131, // SI8
        ];
        for (i, &val) in function_values.iter().enumerate() {
            w.write_sdo_i32(
                Reg::CONTROL_INPUT_SI_FUNCTION_ASSIGNMENT + i as u16,
                0,
                val as i32,
            )
            .await?;
        }
        // Persist to EEPROM
        w.write_sdo_u32(Reg::EEPROM, 1, 0x65766173).await?;
        tracing::info!("EEPROM written with SI input pin configuration.");
        Ok(())
    }
}

// Async SDO configuration (called during pre-op, before OP transition)

impl MinasA6BMotor {
    pub async fn configure<'a>(
        &self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<(), Error> {
        let w = EtherCATSlaveWrapper::new(device);

        w.configure_pdo_mapping(Reg::RX_PDO, &SET_DATA_MAPPING)
            .await?;
        w.configure_pdo_mapping(Reg::TX_PDO, &GET_DATA_MAPPING)
            .await?;
        w.assign_pdos(Reg::RX_PDO_ASSIGN_ADDRESS, &[Reg::RX_PDO])
            .await?;
        w.assign_pdos(Reg::TX_PDO_ASSIGN_ADDRESS, &[Reg::TX_PDO])
            .await?;

        w.write_sdo_u16(Reg::POSITIONING_OPTION, 0, 0).await?;
        w.write_sdo_u32(Reg::POSITION_WINDOW, 0, 10_000).await?;
        w.write_sdo_u16(Reg::POSITION_WINDOW_TIME, 0, 0).await?;
        w.write_sdo_u32(Reg::FOLLOWING_WINDOW, 0, 100).await?;
        w.write_sdo_u16(Reg::FOLLOWING_WINDOW_TIME, 0, 1).await?;
        w.write_sdo_i16(Reg::ENCODER_SETTING, 0, 0).await?; // absolute encoder
        w.write_sdo_u16(Reg::MAX_TORQUE, 0, 500).await?; // 50.0 %

        Ok(())
    }

    /// Read and store the encoder resolution from the drive.
    pub async fn read_encoder_resolution<'a>(
        &mut self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<EncoderResolution, Error> {
        let w = EtherCATSlaveWrapper::new(device);
        let increments = w.read_sdo_u32(Reg::ENCODER_RESOLUTION, 1).await?;
        let revolutions = w.read_sdo_u32(Reg::ENCODER_RESOLUTION, 2).await?;
        if revolutions == 0 {
            return Err(anyhow!(
                "ENCODER_RESOLUTION sub-index 2 returned 0 revolutions"
            ));
        }
        let res = EncoderResolution {
            increments,
            revolutions,
        };
        tracing::info!("Encoder resolution: {}/{} inc/rev", increments, revolutions);
        self.encoder_resolution = Some(res);
        Ok(res)
    }

    /// Write homing parameters to the drive over SDO.
    pub async fn setup_homing<'a>(
        &self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<(), Error> {
        let enc = self
            .encoder_resolution
            .ok_or_else(|| anyhow!("setup_homing called before encoder resolution was read"))?;
        let w = EtherCATSlaveWrapper::new(device);

        let mode_val: u8 = self.homing_config.homing_direction.mode_code();
        w.write_sdo_u8(Reg::HOMING_MODE, 0, mode_val).await?;
        w.write_sdo_u32(
            Reg::HOMING_SPEED,
            1,
            enc.rps_to_inc_per_sec(self.homing_config.high_speed_rps),
        )
        .await?;
        w.write_sdo_u32(
            Reg::HOMING_SPEED,
            2,
            enc.rps_to_inc_per_sec(self.homing_config.slow_speed_rps),
        )
        .await?;
        w.write_sdo_u32(
            Reg::HOMING_ACC,
            0,
            enc.rps_to_inc_per_sec(self.homing_config.acceleration_rps_squared),
        )
        .await?;
        w.write_sdo_i32(Reg::HOMING_OFFSET, 0, 0).await?;
        tracing::info!("Setup Homing done");
        Ok(())
    }

    /// Write SDO registers required to start a multi-turn encoder reset.
    /// Must be called from async context when `state == State::ErrorEncoder`.
    pub async fn start_encoder_multi_turn_reset<'a>(
        &mut self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<(), Error> {
        let w = EtherCATSlaveWrapper::new(device);
        w.write_sdo_u16(Reg::ENCODER_MULTI_TURN_DATA_CLEAR, 0, 0x31)
            .await?;
        w.write_sdo_u32(Reg::ENCODER_MULTI_TURN_DATA_CLEAR_TRIGGER, 1, 1 << 9)
            .await?;
        self.last_fault_reset_toggle
            .get_or_insert_with(Instant::now);
        Ok(())
    }

    /// Poll the clear-status register; returns `true` when reset is complete.
    pub async fn encoder_multi_turn_reset_done<'a>(
        &self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<bool, Error> {
        let w = EtherCATSlaveWrapper::new(device);
        Ok(w.read_sdo_u16(Reg::ENCODER_MULTI_TURN_DATA_CLEAR, 0)
            .await?
            == 0)
    }

    /// Clear the encoder reset trigger registers after the reset cycle completes.
    pub async fn finish_encoder_multi_turn_reset<'a>(
        &mut self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<(), Error> {
        let w = EtherCATSlaveWrapper::new(device);
        w.write_sdo_u32(Reg::ENCODER_MULTI_TURN_DATA_CLEAR_TRIGGER, 1, 0)
            .await?;
        self.last_fault_reset_toggle = None;
        Ok(())
    }
}

// Public motor API
impl MinasA6BMotor {
    pub fn get_position(&self) -> Option<f64> {
        self.encoder_resolution
            .map(|enc| enc.to_revolutions(self.txpdo.position_actual))
    }

    pub fn is_enabled(&self) -> bool {
        self.state.is_run()
    }
    pub fn is_homed(&self) -> bool {
        self.homed
    }
    pub fn is_motion_done(&self) -> bool {
        self.state == State::RunIdle
    }
    pub fn is_shut_down(&self) -> bool {
        self.state == State::PreOpSwitchOnDisabled
    }
    pub fn has_error(&self) -> bool {
        self.state.is_error()
    }

    pub fn enable(&mut self) {
        // Symmetric with disable(): on a repeat call, just drive the state
        // machine forward instead of re-issuing the command immediately.
        if self.single_enable_flag {
            self.poll();
        } else {
            self.single_enable_flag = true;
            self.send_matching_enable_command();
        }
    }

    pub fn disable(&mut self) {
        if self.single_disable_flag {
            self.poll();
        } else {
            self.single_disable_flag = true;
            self.send_matching_disable_command();
        }
    }

    pub async fn clear_alarm<'a>(
        &self,
        device: &'a EthercrabSubDevicePreoperational<'a>,
    ) -> Result<(), Error> {
        let w = EtherCATSlaveWrapper::new(device);
        // Panasonic alarm clear object
        w.write_sdo_u16(0x2280, 0, 0x0001).await?;
        tracing::info!("Alarm clear sent");
        Ok(())
    }

    pub fn quick_stop(&mut self) {
        if self.state.is_run() {
            self.apply_rxpdo(MinasA6BRxPdo {
                control_word: Reg::CW_QUICK_STOP,
                mode: self.current_motor_mode,
                ..Default::default()
            });
            self.state = State::QuickStopWaiting;
        }
    }

    pub fn leave_quick_stop(&mut self) {
        if self.state != State::QuickStopReached {
            return;
        }
        self.rxpdo = MinasA6BRxPdo {
            control_word: 0b1111, // operation enabled
            mode: self.current_motor_mode,
            ..Default::default()
        };
        if self.status_run() {
            self.state = State::RunIdle;
        }
    }

    pub fn move_position(
        &mut self,
        position: PositionSpec,
        transition: Option<PositionTransitionSpec>,
    ) {
        let Some(enc) = self.encoder_resolution else {
            tracing::error!("move_position called before encoder resolution is known");
            return;
        };
        tracing::debug!("Current position: {} inc", self.txpdo.position_actual);
        let pdo = self.build_rxpdo_from_position_spec(&position, transition.as_ref(), enc);
        self.motion_out_deque.push_back(pdo);
        self.motion_transition = transition;
    }

    pub fn move_positions(
        &mut self,
        positions: Vec<PositionSpec>,
        transition: Option<PositionTransitionSpec>,
    ) {
        let Some(enc) = self.encoder_resolution else {
            tracing::error!("move_positions called before encoder resolution is known");
            return;
        };
        for position in positions {
            let pdo = self.build_rxpdo_from_position_spec(&position, transition.as_ref(), enc);
            self.motion_out_deque.push_back(pdo);
        }
        self.motion_transition = transition;
    }

    pub fn abort_motion(&mut self) {
        if let Some(pos) = self.get_position() {
            self.motion_out_deque.clear();
            self.move_position(
                PositionSpec {
                    n_revolutions: pos,
                    is_absolute: true,
                    speed_rps: 1.0,
                    acceleration_rps2: 10.0,
                    deceleration_rps2: 10.0,
                },
                None,
            );
            self.previous_motion_command = None;
            self.current_motion_command = None;
            self.motion_transition = None;
            tracing::info!("Triggering immediate stop of ongoing motion...");
        }
    }

    pub fn home(&mut self) {
        if self.homing_issued.is_some() {
            self.poll();
        } else if matches!(self.state, State::Ready | State::RunIdle) {
            tracing::info!("Triggering homing...");
            self.current_motor_mode = MotorMode::HOMING;
            self.mode_change_issued = Some(Instant::now());
            self.state = State::HomingIdle;
            self.write_keepalive_pdo();
        } else {
            self.enable();
            self.needs_homing = true;
        }
    }

    pub fn shut_down(&mut self) {
        self.disable();
    }
}

// State machine poll()
impl MinasA6BMotor {
    /// Called from `EthercatDevice::input()` once the TxPDO has been parsed.
    pub fn poll(&mut self) {
        match self.state {
            // Re-classify from Unknown / post-reset
            State::Unknown | State::ErrorResetToggled => {
                self.classify_state();
            }

            // CiA 402 pre-op ladder (forward and backward)
            State::PreOpNotRdyToSwitchOn => {
                if self.single_enable_flag {
                    self.send_matching_enable_command();
                }
                if self.status_switch_on_disabled() {
                    self.state = State::PreOpSwitchOnDisabled;
                }
            }
            State::PreOpSwitchOnDisabled => {
                self.single_disable_flag = false;
                if self.single_enable_flag {
                    self.send_matching_enable_command();
                }
                if self.status_ready_to_switch_on() {
                    self.state = State::PreOpReadyToSwitchOn;
                }
            }
            State::PreOpReadyToSwitchOn => {
                if self.single_enable_flag {
                    self.send_matching_enable_command();
                }
                if self.single_disable_flag {
                    self.send_matching_disable_command();
                }
                if self.status_ready() {
                    self.state = State::Ready;
                } else if self.status_switch_on_disabled() {
                    self.state = State::PreOpSwitchOnDisabled;
                }
            }
            State::Ready => {
                if self.single_enable_flag {
                    self.send_matching_enable_command();
                }
                if self.single_disable_flag {
                    self.send_matching_disable_command();
                }
                if self.status_run() {
                    self.single_enable_flag = false;
                    self.state = State::RunIdle;
                } else if self.status_ready_to_switch_on() {
                    self.state = State::PreOpReadyToSwitchOn;
                }
            }

            // Run states
            State::RunIdle => {
                self.single_enable_flag = false;
                self.check_fault();
                if self.needs_homing {
                    self.current_motor_mode = MotorMode::HOMING;
                    self.mode_change_issued = Some(Instant::now());
                    self.write_keepalive_pdo();
                    self.state = State::HomingIdle;
                    return;
                }
                if self.single_disable_flag {
                    self.send_matching_disable_command();
                }
                if self.status_ready() {
                    self.state = State::Ready;
                    return;
                }
                if !self.motion_out_deque.is_empty() {
                    self.send_motion_command();
                    self.state = State::RunSentMotionCommand;
                }
            }
            State::RunSentMotionCommand => {
                self.single_enable_flag = false;
                self.check_fault();
                if self.previous_motion_command.is_none() {
                    self.make_set_point_ack();
                    self.movement_delay = None;
                    self.state = State::RunSentSetPointAck;
                } else {
                    self.state = State::RunWaitForSendingNextCommand;
                }
            }
            State::RunWaitForSendingNextCommand => {
                self.single_enable_flag = false;
                self.check_fault();
                if self.sending_next_command_granted() {
                    self.make_set_point_ack();
                    self.movement_delay = None;
                    self.state = State::RunSentSetPointAck;
                }
            }
            State::RunSentSetPointAck => {
                self.single_enable_flag = false;
                self.check_fault();
                self.make_set_point_nack();
                self.state = State::RunSentSetPointNack;
            }
            State::RunSentSetPointNack => {
                self.single_enable_flag = false;
                self.check_fault();
                if self.status_set_point_nack() {
                    self.state = State::RunExecuting;
                }
            }
            State::RunExecuting => {
                self.single_enable_flag = false;
                self.check_fault();
                if self.status_target_reached() && self.motion_out_deque.is_empty() {
                    self.previous_motion_command = None;
                    self.current_motion_command = None;
                    self.motion_transition = None;
                    self.state = State::RunIdle;
                } else if !self.motion_out_deque.is_empty() {
                    self.send_motion_command();
                    self.state = State::RunSentMotionCommand;
                }
            }

            // Homing
            State::HomingIdle => {
                let mode_ok = self.txpdo.mode_display == MotorMode::HOMING;
                let settled = elapsed_at_least(&self.mode_change_issued, 3);
                if !mode_ok || !settled {
                    self.write_keepalive_pdo();
                } else {
                    // Set start-homing bit (bit 4)
                    self.apply_rxpdo(MinasA6BRxPdo {
                        control_word: Reg::CONTROL_WORD_ENABLE | 0x0010,
                        mode: MotorMode::HOMING,
                        ..Default::default()
                    });
                    self.homing_issued = Some(Instant::now());
                    self.state = State::HomingSendingHomeCommand;
                }
            }
            State::HomingSendingHomeCommand => {
                if elapsed_at_least(&self.homing_issued, 100) {
                    // Clear start-homing bit
                    self.apply_rxpdo(MinasA6BRxPdo {
                        control_word: Reg::CONTROL_WORD_ENABLE,
                        mode: MotorMode::HOMING,
                        ..Default::default()
                    });
                    self.state = State::HomingWaitForFinish;
                }
            }
            State::HomingWaitForFinish => {
                if self.status_homing_finished() {
                    tracing::info!("Homing finished successfully!");
                    self.homed = true;
                    self.finish_homing();
                } else if self.status_homing_failed() {
                    tracing::error!("Homing error occurred!");
                    self.homed = false;
                    self.finish_homing();
                }
            }
            State::HomingWaitForModeChange => {
                let mode_ok = self.txpdo.mode_display == MotorMode::PP;
                let settled = elapsed_at_least(&self.mode_change_issued, 3);
                if mode_ok && settled {
                    self.needs_homing = false;
                    self.state = State::RunIdle;
                }
            }

            // Quick-stop
            State::QuickStopWaiting => {
                if self.status_quick_stop() {
                    self.state = State::QuickStopReached;
                }
            }
            State::QuickStopReached => { /* stays here until leave_quick_stop() */ }

            // Error / fault reset
            State::ErrorAny => {
                if self.error_code_is_encoder() {
                    self.state = State::ErrorEncoder;
                    // Owning task must call start_encoder_multi_turn_reset() async
                    tracing::warn!(
                        "Encoder multi-turn reset required — call start_encoder_multi_turn_reset() from async task"
                    );
                } else {
                    self.apply_rxpdo(MinasA6BRxPdo {
                        control_word: Reg::CW_FAULT_RESET,
                        mode: self.current_motor_mode,
                        ..Default::default()
                    });
                    self.last_fault_reset_toggle
                        .get_or_insert_with(Instant::now);
                    self.state = State::ErrorGeneric;
                }
            }
            State::ErrorEncoder => {
                // Async encoder-reset SDO steps are handled externally.
                // When done, the owning task sets state back to ErrorGeneric
                // via notify_encoder_reset_complete().
            }
            State::ErrorGeneric => {
                // Self-arm if we arrived here without a timer set
                if self.last_fault_reset_toggle.is_none() {
                    self.last_fault_reset_toggle = Some(Instant::now());
                }
                if elapsed_at_least_secs(&self.last_fault_reset_toggle, 0.3) {
                    self.apply_rxpdo(MinasA6BRxPdo {
                        control_word: Reg::CONTROL_WORD_DISABLE,
                        mode: self.current_motor_mode,
                        ..Default::default()
                    });
                    self.last_fault_reset_toggle = None;
                    self.state = State::ErrorResetToggled;
                }
            }
            State::ErrorResetToggled => {
                self.classify_state();
            }
        }
    }

    /// Called by the owning async task once `encoder_multi_turn_reset_done()` returns true.
    pub fn notify_encoder_reset_complete(&mut self) {
        self.last_fault_reset_toggle = None;
        self.needs_homing = true;
        self.homed = false; // encoder was corrupted homing must be repeated
        tracing::warn!("Queuing motor homing as encoder was corrupted...");
        // Re-enter generic fault reset to deassert the bit
        self.apply_rxpdo(MinasA6BRxPdo {
            control_word: Reg::CW_FAULT_RESET,
            mode: self.current_motor_mode,
            ..Default::default()
        });
        self.last_fault_reset_toggle = Some(Instant::now());
        self.state = State::ErrorGeneric;
    }

    // Internal helpers
    fn classify_state(&mut self) {
        if self.status_fault() {
            self.tell_about_error();
            self.state = State::ErrorAny;
        } else if self.status_not_ready_to_switch_on() {
            self.state = State::PreOpNotRdyToSwitchOn;
        } else if self.status_switch_on_disabled() {
            self.state = State::PreOpSwitchOnDisabled;
        } else if self.status_ready_to_switch_on() {
            self.state = State::PreOpReadyToSwitchOn;
        } else if self.status_ready() {
            self.state = State::Ready;
        } else if self.status_run() {
            self.state = State::RunIdle;
        }
    }

    fn check_fault(&mut self) {
        if self.last_fault_reset_toggle.is_none()
            && self.status_fault()
            && self.state != State::ErrorAny
        {
            self.tell_about_error();
            self.state = State::ErrorAny;
        }
    }

    fn finish_homing(&mut self) {
        self.homing_issued = None;
        self.current_motor_mode = MotorMode::PP;
        self.mode_change_issued = Some(Instant::now());
        self.write_keepalive_pdo();
        self.state = State::HomingWaitForModeChange;
    }

    fn apply_rxpdo(&mut self, pdo: MinasA6BRxPdo) {
        self.rxpdo = pdo;
    }

    /// Write a keep-alive PDO (enable control word, current mode, zeroed motion fields).
    fn write_keepalive_pdo(&mut self) {
        self.rxpdo = MinasA6BRxPdo {
            control_word: Reg::CONTROL_WORD_ENABLE,
            mode: self.current_motor_mode,
            ..Default::default()
        };
    }
}

// CiA 402 status word predicates
impl MinasA6BMotor {
    fn sw(&self) -> u16 {
        self.txpdo.status_word
    }

    fn status_fault(&self) -> bool {
        self.sw() & 0x4F == 0x08
    }
    fn status_not_ready_to_switch_on(&self) -> bool {
        self.sw() & 0x4F == 0x00
    }
    fn status_switch_on_disabled(&self) -> bool {
        self.sw() & 0x4F == 0x40
    }
    fn status_ready_to_switch_on(&self) -> bool {
        self.sw() & 0x6F == 0x21
    }
    fn status_ready(&self) -> bool {
        self.sw() & 0x6F == 0x23
    }
    fn status_run(&self) -> bool {
        self.sw() & 0x6F == 0x27
    }
    fn status_quick_stop(&self) -> bool {
        self.sw() & 0x6F == 0x07
    }
    fn status_homing_finished(&self) -> bool {
        self.sw() & 0x1000 == 0x1000
    }
    fn status_homing_failed(&self) -> bool {
        self.sw() & 0x2000 == 0x2000
    }
    fn status_set_point_nack(&self) -> bool {
        self.sw() & 0x1000 == 0x0000
    }
    fn status_target_reached(&self) -> bool {
        self.sw() & (1 << 10) != 0
    }
    fn error_code_is_encoder(&self) -> bool {
        self.txpdo.error_code == 0xFF28
    }
}

// CiA 402 command helpers
impl MinasA6BMotor {
    fn send_matching_enable_command(&mut self) {
        tracing::debug!("Sending enable command for state {:?}", self.state);
        let cw = match self.state {
            State::PreOpSwitchOnDisabled | State::PreOpNotRdyToSwitchOn => {
                Reg::CONTROL_WORD_READY_TO_SWITCH_ON
            }
            State::PreOpReadyToSwitchOn => Reg::CONTROL_WORD_SWITCH_ON,
            State::Ready => Reg::CONTROL_WORD_ENABLE,
            _ => return,
        };
        self.rxpdo.control_word = cw;
        self.rxpdo.mode = self.current_motor_mode;
    }

    fn send_matching_disable_command(&mut self) {
        tracing::debug!("Sending disable command for state {:?}", self.state);
        let cw = match self.state {
            State::Ready => Reg::CONTROL_WORD_READY_TO_SWITCH_ON,
            State::PreOpReadyToSwitchOn => Reg::CONTROL_WORD_DISABLE,
            _ => Reg::CONTROL_WORD_READY_TO_SWITCH_ON,
        };
        self.rxpdo.control_word = cw;
        self.rxpdo.mode = self.current_motor_mode;
    }

    fn send_motion_command(&mut self) {
        self.previous_motion_command = self.current_motion_command.take();
        if let Some(cmd) = self.motion_out_deque.pop_front() {
            tracing::info!(
                "Sending motion command ({} left)...",
                self.motion_out_deque.len()
            );
            self.rxpdo = cmd.clone();
            self.current_motion_command = Some(cmd);
        }
    }

    fn make_set_point_ack(&mut self) {
        if let Some(ref cmd) = self.current_motion_command {
            let mut updated = cmd.clone();
            updated.control_word |= 0x0010; // set new-setpoint bit
            tracing::info!(
                "Starting motion command ({} left)...",
                self.motion_out_deque.len()
            );
            self.rxpdo = updated;
        }
    }

    fn make_set_point_nack(&mut self) {
        self.rxpdo = MinasA6BRxPdo {
            control_word: Reg::CONTROL_WORD_ENABLE,
            mode: self.current_motor_mode,
            ..Default::default()
        };
    }

    fn tell_about_error(&self) {
        tracing::error!(
            "Motor fault! {} — trying auto-reset...",
            interpret_error_code(self.txpdo.error_code)
        );
    }

    fn build_rxpdo_from_position_spec(
        &mut self,
        spec: &PositionSpec,
        transition: Option<&PositionTransitionSpec>,
        enc: EncoderResolution,
    ) -> MinasA6BRxPdo {
        // Pass is_absolute so residue is handled correctly
        let target_position = enc.to_increments(
            spec.n_revolutions,
            &mut self.pulse_residue,
            spec.is_absolute,
        );
        let target_velocity = enc.rps_to_inc_per_sec(spec.speed_rps);
        let target_accel = enc.rps_to_inc_per_sec(spec.acceleration_rps2);
        let target_decel = enc.rps_to_inc_per_sec(spec.deceleration_rps2);

        let mut control_word = Reg::CONTROL_WORD_ENABLE;
        if !spec.is_absolute {
            control_word |= 1 << 6;
        }
        if transition.map(|t| t.overlap).unwrap_or(false) {
            control_word |= 1 << 5;
        }

        MinasA6BRxPdo {
            control_word,
            mode: MotorMode::PP,
            target_position,
            target_velocity,
            target_accel,
            target_decel,
        }
    }

    fn sending_next_command_granted(&mut self) -> bool {
        let Some(prev) = self.previous_motion_command.clone() else {
            return true;
        };
        let Some(ref transition) = self.motion_transition.clone() else {
            return self.status_target_reached();
        };

        if transition.overlap {
            let offset = (self.txpdo.position_actual - prev.target_position).unsigned_abs() as f64;
            let v = prev.target_velocity as f64;
            let d = prev.target_decel as f64;
            if d == 0.0 {
                return false;
            }
            let vd = v / d;
            let s_dec = vd * v / 2.0;
            let remaining_ms = if offset >= s_dec {
                1000.0 * (offset / v + vd / 2.0)
            } else {
                let disc = vd * vd - 2.0 * offset / d;
                if disc < 0.0 {
                    return false;
                }
                1000.0 * (vd - disc.sqrt())
            };
            tracing::debug!("remaining_motion_time_ms: {:.1}", remaining_ms);
            remaining_ms <= transition.delay_ms
        } else {
            match self.movement_delay {
                Some(t) => t.elapsed().as_millis() as f64 >= transition.delay_ms,
                None => {
                    if (self.txpdo.position_actual - prev.target_position).unsigned_abs() <= 1000 {
                        self.movement_delay = Some(Instant::now());
                    }
                    false
                }
            }
        }
    }
}

// Small utilities
fn elapsed_at_least(t: &Option<Instant>, millis: u64) -> bool {
    t.map(|i| i.elapsed() >= Duration::from_millis(millis))
        .unwrap_or(false)
}

fn elapsed_at_least_secs(t: &Option<Instant>, secs: f64) -> bool {
    t.map(|i| i.elapsed().as_secs_f64() >= secs)
        .unwrap_or(false)
}

fn interpret_error_code(code: u16) -> String {
    match code {
        0xFF28 => "Encoder multi-turn data error".to_string(),
        c if c & 0xFF00 == 0xFF00 => format!("Error 0x{:02X} ({})", c & 0x00FF, c & 0x00FF),
        c => format!("Error 0x{:04X} ({})", c, c),
    }
}

impl MinasA6BDevice for MinasA6BMotor {
    fn get_input(&self) -> Result<MinasA6BInput, Error> {
        Ok(MinasA6BInput {
            position: self.get_position(),
            is_enabled: self.is_enabled(),
            is_homed: self.is_homed(),
            is_motion_done: self.is_motion_done(),
            is_shut_down: self.is_shut_down(),
            has_error: self.has_error(),
            status_word: self.txpdo.status_word,
            error_code: self.txpdo.error_code,
            state: self.state,
            mode_display: self.txpdo.mode_display,
        })
    }

    fn get_output(&self) -> Result<MinasA6BOutput, Error> {
        Ok(MinasA6BOutput {
            control_word: self.rxpdo.control_word,
            mode: self.rxpdo.mode,
            target_position: self.rxpdo.target_position,
            target_velocity: self.rxpdo.target_velocity,
            target_accel: self.rxpdo.target_accel,
            target_decel: self.rxpdo.target_decel,
        })
    }

    fn enable(&mut self) -> Result<(), Error> {
        MinasA6BMotor::enable(self);
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Error> {
        MinasA6BMotor::disable(self);
        Ok(())
    }

    fn quick_stop(&mut self) -> Result<(), Error> {
        MinasA6BMotor::quick_stop(self);
        Ok(())
    }

    fn leave_quick_stop(&mut self) -> Result<(), Error> {
        MinasA6BMotor::leave_quick_stop(self);
        Ok(())
    }

    fn move_position(
        &mut self,
        position: PositionSpec,
        transition: Option<PositionTransitionSpec>,
    ) -> Result<(), Error> {
        self.move_position(position, transition);
        Ok(())
    }

    fn move_positions(
        &mut self,
        positions: Vec<PositionSpec>,
        transition: Option<PositionTransitionSpec>,
    ) -> Result<(), Error> {
        self.move_positions(positions, transition);
        Ok(())
    }

    fn abort_motion(&mut self) -> Result<(), Error> {
        self.abort_motion();
        Ok(())
    }

    fn home(&mut self) -> Result<(), Error> {
        self.home();
        Ok(())
    }
}

pub const MINAS_A6_VENDOR_ID: u32 = 0x66f;
pub const MINAS_A6_PRODUCT_ID_A: u32 = 0x613c0006;
pub const MINAS_A6_REVISION_A: u32 = 0x10000;
pub const MINAS_A6_IDENTITY_A: SubDeviceIdentityTuple = (
    MINAS_A6_VENDOR_ID,
    MINAS_A6_PRODUCT_ID_A,
    MINAS_A6_REVISION_A,
);
