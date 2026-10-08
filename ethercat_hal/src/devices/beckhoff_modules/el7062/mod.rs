use coe::EL7062Configuration;
use ethercat_hal_derive::EthercatDevice;

use std::{time::Duration, time::Instant};

use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::EtherCATThreadChannel;
use crate::pdo::{PredefinedPdoAssignment, RxPdo, TxPdo};
use anyhow::anyhow;

pub mod coe;
pub mod motion;
pub mod pdo;

/// How long one fault-reset pulse holds CiA 402 controlword bit 7 high.
///
/// The reset acts on the bit's rising edge. After the hold, the bit drops so
/// the next faulted cycle can raise a fresh edge — see
/// [`Axis::apply_controlword`].
const FAULT_RESET_HOLD: Duration = Duration::from_millis(300);

/// Runtime bookkeeping per channel, owned by the device rather than the
/// transient [`Axis`] borrow (which is recreated every cycle).
#[derive(Debug, Default, Clone)]
struct AxisState {
    /// When the current fault-reset pulse was asserted, i.e. how far it has
    /// held. `None` means bit 7 is low and the next faulted cycle starts a
    /// fresh pulse.
    fault_reset_asserted: Option<Instant>,

    /// Whether the application wrote a setpoint (target position, velocity or
    /// torque) since the last fault. `apply_controlword` refuses
    /// `enable_operation` until this is set again, so an enable never runs a
    /// stale setpoint.
    target_set: bool,
}

#[derive(EthercatDevice, Clone, Debug)]
pub struct EL7062 {
    pub txpdo: pdo::EL7062TxPdo,
    pub rxpdo: pdo::EL7062RxPdo,
    is_used: bool,
    pub configuration: EL7062Configuration,
    /// Runtime state that outlives the per-cycle [`Axis`] borrow: the
    /// fault-reset pulse and the setpoint latch, per channel.
    ch1_state: AxisState,
    ch2_state: AxisState,
}

impl NewEthercatDevice for EL7062 {
    fn new() -> Self {
        let configuration = EL7062Configuration::default();
        Self {
            txpdo: configuration.pdo_assignment.txpdo_assignment(),
            rxpdo: configuration.pdo_assignment.rxpdo_assignment(),
            is_used: false,
            configuration,
            ch1_state: AxisState::default(),
            ch2_state: AxisState::default(),
        }
    }
}

impl EthercatDeviceProcessing for EL7062 {}

/// Port selector for the two channels of the EL7062.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EL7062Port {
    Ch1,
    Ch2,
}

/// One channel of the EL7062, mutably borrowed from the device.
///
/// Borrowing the whole device means only one `Axis` exists at a time, so a
/// getter and a setter cannot end up on different channels.
pub struct Axis<'a> {
    channel: EL7062Port,
    /// The channel's own configuration, used to interpret process data.
    config: &'a coe::El7062ChannelConfiguration,
    position: &'a Option<pdo::FbPosition>,
    statusword: &'a Option<pdo::DrvStatusWord>,
    following_error: &'a Option<pdo::DrvFollowingError>,
    digital_inputs: &'a Option<pdo::DiInputs>,
    velocity_actual: &'a Option<pdo::DrvVelocityActual>,
    torque_actual: &'a Option<pdo::DrvTorqueActual>,
    mode_display: &'a Option<pdo::DrvModeOfOperationDisplay>,
    info_data_1: &'a Option<pdo::DrvInfoData>,
    info_data_2: &'a Option<pdo::DrvInfoData>,
    info_data_3: &'a Option<pdo::DrvInfoData>,
    /// The channel's runtime state (fault-reset pulse, setpoint latch).
    state: &'a mut AxisState,
    /// The selected channel's RxPDO slots, held as `&mut` alongside the `&`
    /// fields above so setters can write while getters still read.
    rx: RxPdoSlot<'a>,
}

enum RxPdoSlot<'a> {
    Ch1 {
        controlword: &'a mut Option<pdo::DrvControlWord>,
        position: &'a mut Option<pdo::DrvTargetPosition>,
        target_velocity: &'a mut Option<pdo::DrvTargetVelocity>,
        target_torque: &'a mut Option<pdo::DrvTargetTorque>,
        commutation_angle: &'a mut Option<pdo::DrvCommutationAngle>,
    },
    Ch2 {
        controlword: &'a mut Option<pdo::DrvControlWord>,
        position: &'a mut Option<pdo::DrvTargetPosition>,
        target_velocity: &'a mut Option<pdo::DrvTargetVelocity>,
        target_torque: &'a mut Option<pdo::DrvTargetTorque>,
        commutation_angle: &'a mut Option<pdo::DrvCommutationAngle>,
    },
}

impl EL7062 {
    /// Borrow one channel for reading and writing process data.
    pub fn axis(&mut self, port: EL7062Port) -> Axis<'_> {
        let config = match port {
            EL7062Port::Ch1 => &self.configuration.channel_1,
            EL7062Port::Ch2 => &self.configuration.channel_2,
        };
        let (
            position,
            statusword,
            following_error,
            digital_inputs,
            velocity_actual,
            torque_actual,
            mode_display,
            info_data_1,
            info_data_2,
            info_data_3,
        ) = match port {
            EL7062Port::Ch1 => (
                &self.txpdo.ch1_position,
                &self.txpdo.ch1_statusword,
                &self.txpdo.ch1_following_error,
                &self.txpdo.ch1_digital_inputs,
                &self.txpdo.ch1_velocity_actual,
                &self.txpdo.ch1_torque_actual,
                &self.txpdo.ch1_mode_display,
                &self.txpdo.ch1_info_data_1,
                &self.txpdo.ch1_info_data_2,
                &self.txpdo.ch1_info_data_3,
            ),
            EL7062Port::Ch2 => (
                &self.txpdo.ch2_position,
                &self.txpdo.ch2_statusword,
                &self.txpdo.ch2_following_error,
                &self.txpdo.ch2_digital_inputs,
                &self.txpdo.ch2_velocity_actual,
                &self.txpdo.ch2_torque_actual,
                &self.txpdo.ch2_mode_display,
                &self.txpdo.ch2_info_data_1,
                &self.txpdo.ch2_info_data_2,
                &self.txpdo.ch2_info_data_3,
            ),
        };
        let state = match port {
            EL7062Port::Ch1 => &mut self.ch1_state,
            EL7062Port::Ch2 => &mut self.ch2_state,
        };
        let rx = match port {
            EL7062Port::Ch1 => RxPdoSlot::Ch1 {
                controlword: &mut self.rxpdo.ch1_controlword,
                position: &mut self.rxpdo.ch1_position,
                target_velocity: &mut self.rxpdo.ch1_target_velocity,
                target_torque: &mut self.rxpdo.ch1_target_torque,
                commutation_angle: &mut self.rxpdo.ch1_commutation_angle,
            },
            EL7062Port::Ch2 => RxPdoSlot::Ch2 {
                controlword: &mut self.rxpdo.ch2_controlword,
                position: &mut self.rxpdo.ch2_position,
                target_velocity: &mut self.rxpdo.ch2_target_velocity,
                target_torque: &mut self.rxpdo.ch2_target_torque,
                commutation_angle: &mut self.rxpdo.ch2_commutation_angle,
            },
        };
        Axis {
            channel: port,
            config,
            position,
            statusword,
            following_error,
            digital_inputs,
            velocity_actual,
            torque_actual,
            mode_display,
            info_data_1,
            info_data_2,
            info_data_3,
            state,
            rx,
        }
    }

    /// The diagnosis of *why* an axis faulted. These are SDO reads, so call
    /// them from the application task (on a fault transition, say), not
    /// inside the cycle loop. The driver also carries the realtime answers in
    /// process data: [`Axis::info_data_1`] with its default selection is the
    /// DC-link voltage every cycle, and [`Axis::following_error`] shows the
    /// axis losing step by step.
    ///
    /// The one thing these cannot deliver is the 0x10F3 diagnosis history:
    /// its messages are 32-byte strings, which the typed SDO interface of
    /// [`crate::EtherCATThreadChannel`] cannot read today.
    pub fn read_dc_link_voltage(
        channel: &EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<u32, anyhow::Error> {
        channel.sdo_read(device_address, 0xF900, 0x12)
    }

    /// Read the supply voltage Up (0xF900:13), in mV.
    pub fn read_supply_voltage(
        channel: &EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<u32, anyhow::Error> {
        channel.sdo_read(device_address, 0xF900, 0x13)
    }

    /// Read the amplifier temperature (0xF900:11), in 0.1 °C.
    pub fn read_amplifier_temperature(
        channel: &EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<i16, anyhow::Error> {
        channel.sdo_read(device_address, 0xF900, 0x11)
    }

    /// Read the power stage state of one channel
    /// (0x9010:27 / 0x9110:27): `0` = safe state, `1` = ready state.
    ///
    /// A faulted output stage sits in `safe_state`; together with a
    /// `statusword.fault` this tells "fault latched on the axis" apart from
    /// "the output stage itself is the fault".
    pub fn read_output_stage_state(
        channel: &EtherCATThreadChannel,
        device_address: u16,
        port: EL7062Port,
    ) -> Result<u8, anyhow::Error> {
        let index = match port {
            EL7062Port::Ch1 => 0x9010,
            EL7062Port::Ch2 => 0x9110,
        };
        channel.sdo_read(device_address, index, 0x27)
    }

    /// Read the channel's motor brake state
    /// (0x9010:28 / 0x9110:28): `0` = applied, `1` = released.
    pub fn read_brake_state(
        channel: &EtherCATThreadChannel,
        device_address: u16,
        port: EL7062Port,
    ) -> Result<u8, anyhow::Error> {
        let index = match port {
            EL7062Port::Ch1 => 0x9010,
            EL7062Port::Ch2 => 0x9110,
        };
        channel.sdo_read(device_address, index, 0x28)
    }
}

impl Axis<'_> {
    /// Which channel this is.
    pub fn port(&self) -> EL7062Port {
        self.channel
    }

    /// The channel's configuration.
    pub fn config(&self) -> &coe::El7062ChannelConfiguration {
        self.config
    }

    /// The channel's statusword monitor, for decoding bit 10.
    pub fn statusword_monitor(&self) -> pdo::StatuswordProcessDataMonitor {
        self.config.amplifier.statusword_monitor
    }

    /// Get the position feedback, in process-data increments.
    pub fn position(&self) -> Result<i32, anyhow::Error> {
        self.position
            .as_ref()
            .map(|p| p.position)
            .ok_or_else(|| anyhow!("Position PDO of channel {:?} is None", self.channel))
    }

    /// Get the status word.
    pub fn statusword(&self) -> Result<pdo::DrvStatusWord, anyhow::Error> {
        self.statusword
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("Statusword PDO of channel {:?} is None", self.channel))
    }

    /// Get the following error, in process-data increments.
    ///
    /// Only mapped in CSP; other modes leave it `None`.
    pub fn following_error(&self) -> Result<i32, anyhow::Error> {
        self.following_error
            .as_ref()
            .map(|f| f.following_error)
            .ok_or_else(|| anyhow!("Following error PDO of channel {:?} is None", self.channel))
    }

    /// Get the channel's digital inputs (limit switch inputs 1 and 2).
    pub fn digital_inputs(&self) -> Result<pdo::DiInputs, anyhow::Error> {
        self.digital_inputs
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("Digital inputs PDO of channel {:?} is None", self.channel))
    }

    /// Get the actual velocity (0x6010:07), in increments per second.
    ///
    /// Mapped in every mode, positive towards increasing position.
    pub fn velocity_actual(&self) -> Result<i32, anyhow::Error> {
        self.velocity_actual
            .as_ref()
            .map(|v| v.velocity_actual)
            .ok_or_else(|| anyhow!("Velocity actual PDO of channel {:?} is None", self.channel))
    }

    /// Get the actual torque (0x6010:08), in thousandths of the channel's
    /// rated current `0x8011:12`.
    ///
    /// Only mapped in CST and CSTCA; other modes leave it `None`.
    pub fn torque_actual(&self) -> Result<i16, anyhow::Error> {
        self.torque_actual
            .as_ref()
            .map(|t| t.torque_actual)
            .ok_or_else(|| {
                anyhow!(
                    "Torque actual PDO of channel {:?} is None (CST/CSTCA only)",
                    self.channel
                )
            })
    }

    /// Get the mode the drive actually runs (0x6010:03).
    ///
    /// The mode written at configuration time is a request; this is what the
    /// terminal reports it is doing. Decoded from the raw value; unknown
    /// values survive as [`pdo::DriveModeDisplay::Unknown`].
    pub fn mode_of_operation_display(&self) -> Result<pdo::DriveModeDisplay, anyhow::Error> {
        let raw = self
            .mode_display
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "Modes of operation display PDO of channel {:?} is None",
                    self.channel
                )
            })?
            .mode;
        Ok(pdo::DriveModeDisplay::from_raw(raw))
    }

    /// Get "Info data 1", decoded per the channel's `info_data_1` selection.
    /// Default selection: DC-link voltage in mV.
    pub fn info_data_1(&self) -> Result<pdo::InfoDataValue, anyhow::Error> {
        let raw = self.info_data_raw(self.info_data_1, 1)?;
        Ok(pdo::DrvInfoData { value: raw }.value(self.config.amplifier.info_data_1))
    }

    /// Get "Info data 2", decoded per the channel's `info_data_2` selection.
    ///
    /// Default selection: PCB temperature in 0.1 °C.
    pub fn info_data_2(&self) -> Result<pdo::InfoDataValue, anyhow::Error> {
        let raw = self.info_data_raw(self.info_data_2, 2)?;
        Ok(pdo::DrvInfoData { value: raw }.value(self.config.amplifier.info_data_2))
    }

    /// Get "Info data 3", decoded per the channel's `info_data_3` selection.
    ///
    /// Default selection: the channel's digital inputs in the
    /// [`pdo::DiInputs`] bit layout.
    pub fn info_data_3(&self) -> Result<pdo::InfoDataValue, anyhow::Error> {
        let raw = self.info_data_raw(self.info_data_3, 3)?;
        Ok(pdo::DrvInfoData { value: raw }.value(self.config.amplifier.info_data_3))
    }

    /// The raw bits of one of the three info-data TxPDOs.
    fn info_data_raw(
        &self,
        field: &Option<pdo::DrvInfoData>,
        which: u8,
    ) -> Result<u16, anyhow::Error> {
        field.as_ref().map(|d| d.value).ok_or_else(|| {
            anyhow!(
                "Info data {which} PDO of channel {:?} is None",
                self.channel
            )
        })
    }

    /// Set the control word.
    pub fn set_controlword(
        &mut self,
        controlword: pdo::DrvControlWord,
    ) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 { controlword, .. } => controlword,
            RxPdoSlot::Ch2 { controlword, .. } => controlword,
        };
        match field {
            Some(value) => {
                *value = controlword;
                Ok(())
            }
            None => Err(anyhow!(
                "Controlword PDO of channel {:?} is None",
                self.channel
            )),
        }
    }

    /// Set the target position, in process-data increments.
    ///
    /// Marks the setpoint as written, which lets
    /// [`Axis::apply_controlword`] enable operation; see [`AxisState`].
    ///
    /// Only mapped in CSP; other modes leave it `None`.
    pub fn set_target_position(&mut self, position: i32) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 { position, .. } => position,
            RxPdoSlot::Ch2 { position, .. } => position,
        };
        match field {
            Some(value) => {
                value.target_position = position;
                self.state.target_set = true;
                Ok(())
            }
            None => Err(anyhow!(
                "Target position PDO of channel {:?} is None (CSP only)",
                self.channel
            )),
        }
    }

    /// Set the target velocity (CSV mode), in increments per second.
    ///
    /// Marks the setpoint as written, like [`Axis::set_target_position`].
    pub fn set_target_velocity(&mut self, velocity: i32) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 {
                target_velocity, ..
            } => target_velocity,
            RxPdoSlot::Ch2 {
                target_velocity, ..
            } => target_velocity,
        };
        match field {
            Some(value) => {
                value.target_velocity = velocity;
                self.state.target_set = true;
                Ok(())
            }
            None => Err(anyhow!(
                "Target velocity PDO of channel {:?} is None (CSV only)",
                self.channel
            )),
        }
    }

    /// Set the target torque (CST / CSTCA mode), in thousandths of the
    /// channel's rated current `0x8011:12`.
    ///
    /// Marks the setpoint as written, like [`Axis::set_target_position`].
    pub fn set_target_torque(&mut self, torque: i16) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 { target_torque, .. } => target_torque,
            RxPdoSlot::Ch2 { target_torque, .. } => target_torque,
        };
        match field {
            Some(value) => {
                value.target_torque = torque;
                self.state.target_set = true;
                Ok(())
            }
            None => Err(anyhow!(
                "Target torque PDO of channel {:?} is None (CST/CSTCA only)",
                self.channel
            )),
        }
    }

    /// Set the commutation angle (CSTCA mode only), in electrical degrees
    /// 0-359.
    pub fn set_commutation_angle(&mut self, angle: u16) -> Result<(), anyhow::Error> {
        let field = match &mut self.rx {
            RxPdoSlot::Ch1 {
                commutation_angle, ..
            } => commutation_angle,
            RxPdoSlot::Ch2 {
                commutation_angle, ..
            } => commutation_angle,
        };
        match field {
            Some(value) => {
                value.commutation_angle = angle;
                Ok(())
            }
            None => Err(anyhow!(
                "Commutation angle PDO of channel {:?} is None (CSTCA only)",
                self.channel
            )),
        }
    }

    /// Step the drive towards `Operation enabled`.
    ///
    /// Call once per cycle with the channel's latest status word. A set
    /// `statusword.fault` is handled here, not left to the caller, with an
    /// edge-correct reset: the fault-reset bit (bit 7) is asserted for
    /// [`FAULT_RESET_HOLD`], then dropped, and re-asserted on the next faulted
    /// cycle if the fault persists. CiA 402 resets on the bit's rising edge,
    /// so a bit held high gives exactly one reset attempt.
    ///
    /// Operation is only requested once the application has written a
    /// setpoint since the last fault (`set_target_position`,
    /// `set_target_velocity` or `set_target_torque` mark that). Enabling
    /// earlier would head a CSP axis straight to its stale target-position
    /// PDO value — 0 unless someone wrote to it.
    pub fn apply_controlword(
        &mut self,
        statusword: &pdo::DrvStatusWord,
    ) -> Result<(), anyhow::Error> {
        if statusword.fault {
            if self.state.fault_reset_asserted.is_some() {
                // Pulse in progress: keep bit 7 high until the hold is over.
            } else {
                tracing::warn!(
                    "EL7062 channel {:?} faulted (status word 0x{:04X}); pulsing fault reset",
                    self.channel,
                    statusword.as_raw()
                );
                self.state.fault_reset_asserted = Some(Instant::now());
                // The setpoint a faulty run was using is stale; a fresh one
                // must be written before the axis is enabled again.
                self.state.target_set = false;
            }
            self.set_controlword(pdo::DrvControlWord {
                fault_reset: true,
                ..Default::default()
            })?;
            if self
                .state
                .fault_reset_asserted
                .is_some_and(|asserted| asserted.elapsed() >= FAULT_RESET_HOLD)
            {
                // End of one pulse: drop bit 7 so the next faulted cycle
                // carries a fresh rising edge.
                self.set_controlword(pdo::DrvControlWord::default())?;
                self.state.fault_reset_asserted = None;
            }
            return Ok(());
        }

        // No fault: the pulse is over for good, and the next fault starts a
        // fresh edge.
        self.state.fault_reset_asserted = None;

        if !statusword.ready_to_switch_on {
            self.set_controlword(pdo::DrvControlWord {
                enable_voltage: true,
                quick_stop: true,
                ..Default::default()
            })?;
        } else if !statusword.switched_on {
            self.set_controlword(pdo::DrvControlWord {
                switch_on: true,
                enable_voltage: true,
                quick_stop: true,
                ..Default::default()
            })?;
        } else if self.state.target_set {
            self.set_controlword(pdo::DrvControlWord {
                switch_on: true,
                enable_voltage: true,
                quick_stop: true,
                enable_operation: true,
                ..Default::default()
            })?;
        } else {
            // Switched on but no setpoint yet: hold here instead of enabling
            // the power stage under an untracked target.
            self.set_controlword(pdo::DrvControlWord {
                switch_on: true,
                enable_voltage: true,
                quick_stop: true,
                ..Default::default()
            })?;
        }
        Ok(())
    }
}

pub const EL7062_VENDOR_ID: u32 = 0x2;
pub const EL7062_PRODUCT_ID: u32 = 0x1b963052;
pub const EL7062_REVISION_A: u32 = 0x00100000;
pub const EL7062_IDENTITY_A: SubDeviceIdentityTuple =
    (EL7062_VENDOR_ID, EL7062_PRODUCT_ID, EL7062_REVISION_A);

#[cfg(test)]
mod tests {
    use super::*;

    /// Apply one control-word cycle and report what the RxPDO ended up with.
    fn apply_port(
        el: &mut EL7062,
        port: EL7062Port,
        statusword: &pdo::DrvStatusWord,
    ) -> pdo::DrvControlWord {
        {
            let mut axis = el.axis(port);
            axis.apply_controlword(statusword).unwrap();
        }
        let controlword = match port {
            EL7062Port::Ch1 => &el.rxpdo.ch1_controlword,
            EL7062Port::Ch2 => &el.rxpdo.ch2_controlword,
        };
        controlword.clone().unwrap()
    }

    /// Apply one control-word cycle on channel 1 and report the RxPDO.
    fn apply(el: &mut EL7062, statusword: &pdo::DrvStatusWord) -> pdo::DrvControlWord {
        apply_port(el, EL7062Port::Ch1, statusword)
    }

    /// Backdate the channel's pulse clock, as if the hold had expired.
    fn expire_pulse(el: &mut EL7062) {
        let instant = Instant::now() - FAULT_RESET_HOLD;
        el.ch1_state.fault_reset_asserted = Some(instant);
    }

    /// A faulted axis: bit 3 only, as `0x4411` latching looks from the wire.
    fn faulted() -> pdo::DrvStatusWord {
        pdo::DrvStatusWord::from_raw(1 << 3)
    }

    /// Ready to switch on, switched on, not yet operation enabled: bits 0
    /// and 1 of the status word.
    fn switched_on() -> pdo::DrvStatusWord {
        pdo::DrvStatusWord::from_raw(0b0011)
    }

    /// The reset acts on bit 7's rising edge, so holding it high forever
    /// gives exactly one attempt: once in, a fault whose cause persists (a
    /// DC-link undervoltage) would never clear. The pulses must alternate.
    #[test]
    fn fault_reset_pulses_instead_of_latching_the_bit_high() {
        let mut el = EL7062::new();

        // 1st faulted cycle: the edge is raised.
        assert!(apply(&mut el, &faulted()).fault_reset);
        // Well inside the hold: the bit stays high.
        assert!(apply(&mut el, &faulted()).fault_reset);

        // Hold expired: the pulse ends, bit 7 drops.
        expire_pulse(&mut el);
        assert!(!apply(&mut el, &faulted()).fault_reset);

        // Still faulted: a fresh pulse on a fresh edge.
        assert!(apply(&mut el, &faulted()).fault_reset);
    }

    /// A clearing fault ends the bookkeeping without a stray edge afterwards.
    #[test]
    fn the_pulse_ends_when_the_fault_clears() {
        let mut el = EL7062::new();

        assert!(apply(&mut el, &faulted()).fault_reset);
        // Fault cause goes away mid-pulse; the enable path must not keep
        // bit 7 high.
        assert!(!apply(&mut el, &switched_on()).fault_reset);
        // And the next fault starts with a fresh edge, not a continuing arm.
        assert!(apply(&mut el, &faulted()).fault_reset);
    }

    /// Enabling without a setpoint would run a CSP axis to the stale target
    /// position PDO value, so enable_operation waits for a fresh one.
    #[test]
    fn enable_operation_waits_for_a_setpoint() {
        let mut el = EL7062::new();
        let cw = apply(&mut el, &switched_on());
        assert!(cw.switch_on && cw.enable_voltage && cw.quick_stop);
        assert!(!cw.enable_operation, "no setpoint was written");

        {
            let mut axis = el.axis(EL7062Port::Ch1);
            axis.set_target_position(1_048).unwrap();
        }
        let cw = apply(&mut el, &switched_on());
        assert!(cw.enable_operation, "a setpoint was written");
    }

    /// A fault invalidates the setpoint mark: after recovery, enabling again
    /// needs a fresh setpoint, not the one the faulty run was using.
    #[test]
    fn a_fault_withdraws_the_setpoint_mark() {
        let mut el = EL7062::new();
        {
            let mut axis = el.axis(EL7062Port::Ch1);
            axis.set_target_position(1_048).unwrap();
        }
        assert!(apply(&mut el, &switched_on()).enable_operation);

        // Fault: pulse starts and the mark is gone, even though the caller
        // has not written anything new.
        apply(&mut el, &faulted());
        expire_pulse(&mut el);
        apply(&mut el, &faulted()); // pulse ends
        let cw = apply(&mut el, &switched_on());
        assert!(!cw.enable_operation, "post-fault setpoint is stale");
    }

    /// The latch has no channel-specific knowledge of setpoint type: a CSV
    /// velocity and a CST torque both mark it.
    #[test]
    fn every_setpoint_kind_marks_the_channel_set() {
        let mut el = EL7062::new();

        // CSV: the velocity field is only Some with the CSV assignment, so
        // hand it one.
        el.rxpdo.ch1_target_velocity = Some(pdo::DrvTargetVelocity::default());
        {
            let mut axis = el.axis(EL7062Port::Ch1);
            axis.set_target_velocity(-500).unwrap();
        }
        assert!(apply(&mut el, &switched_on()).enable_operation);

        // CST: same, for the torque field.
        el.rxpdo.ch1_target_torque = Some(pdo::DrvTargetTorque::default());
        {
            let mut axis = el.axis(EL7062Port::Ch1);
            axis.set_target_torque(1000).unwrap();
        }
        assert!(apply(&mut el, &switched_on()).enable_operation);
    }

    /// The two channels bookkeep independently: a fault pulse on channel 1
    /// must not withdraw channel 2's setpoint mark.
    #[test]
    fn channels_latch_independently() {
        let mut el = EL7062::new();
        // Only channel 2 gets a setpoint.
        {
            let mut axis = el.axis(EL7062Port::Ch2);
            axis.set_target_position(500).unwrap();
        }
        assert!(apply_port(&mut el, EL7062Port::Ch2, &switched_on()).enable_operation);
        let cw = apply_port(&mut el, EL7062Port::Ch1, &switched_on());
        assert!(!cw.enable_operation, "channel 1 has no setpoint of its own");

        // Channel 1 faults and pulses; that bookkeeping must stay on its own
        // side of the fence.
        apply(&mut el, &faulted());
        expire_pulse(&mut el);
        apply(&mut el, &faulted());
        let cw = apply_port(&mut el, EL7062Port::Ch2, &switched_on());
        assert!(
            cw.enable_operation,
            "channel 1's fault must not touch channel 2's setpoint mark"
        );
    }
}
