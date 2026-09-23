use crate::pdo::{PredefinedPdoAssignment, RxPdoObject, TxPdoObject};
use bitvec::prelude::*;
use ethercat_hal_derive::{PdoObject, RxPdo, TxPdo};

// TxPDO objects (slave -> master)
//
// All TxPDO maps for the EL7062 are derived from the `DRV`/`FB` input object
// dictionary. Channel 1 and channel 2 reuse the same payload structs (they
// point at 0x6000/0x6010 and 0x6100/0x6110 respectively).

/// DRV Inputs Statusword (0x6010:01 Ch.1 / 0x6110:01 Ch.2)
///
/// CiA 402 compatible bits:
///   Bit 0 : Ready to switch on
///   Bit 1 : Switched on
///   Bit 2 : Operation enabled
///   Bit 3 : Fault
///   Bit 6 : Switch on disabled
///   Bit 7 : Warning
/// EL7062 specific bits:
///   Bit 10 : TxPDOToggle
///   Bit 11 : Internal limit active
///   Bit 12 : Drive follows the command value
///   Bit 13 : Input cycle counter
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct DrvStatusWord {
    pub status_word: u16,
}

impl TxPdoObject for DrvStatusWord {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.status_word = bits[0..16].load_le();
    }
}

/// DRV Inputs Modes of operation display (0x6010:03 Ch.1 / 0x6110:03 Ch.2)
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 8)]
pub struct DrvModeOfOperationDisplay {
    pub mode_display: u8,
}

impl TxPdoObject for DrvModeOfOperationDisplay {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.mode_display = bits[0..8].load_le();
    }
}

/// DRV Inputs Following error actual value (0x6010:06 Ch.1 / 0x6110:06 Ch.2)
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct DrvFollowingError {
    pub following_error: i32,
}

impl TxPdoObject for DrvFollowingError {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.following_error = bits[0..32].load_le();
    }
}

/// FB Inputs Position (0x6000:17 Ch.1 / 0x6100:17 Ch.2)
///
/// Unsigned 32-bit position that wraps around modulo 2^32.
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct FbPosition {
    pub position: u32,
}

impl TxPdoObject for FbPosition {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        self.position = bits[0..32].load_le();
    }
}

// RxPDO objects (master -> slave)
//
// The `DRV` output objects live at 0x7010 (Ch.1) / 0x7020 (Ch.2).

/// DRV Outputs Controlword (0x7010:01 Ch.1 / 0x7020:01 Ch.2)
///
/// CiA 402 compatible bits:
///   Bit 0 : Switch on
///   Bit 1 : Enable voltage
///   Bit 2 : reserved
///   Bit 3 : Enable operation
///   Bit 4 - 6 : reserved
///   Bit 7 : Fault reset
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 16)]
pub struct DrvControlWord {
    pub control_word: u16,
}

impl RxPdoObject for DrvControlWord {
    fn write(&self, buffer: &mut BitSlice<u8, Lsb0>) {
        buffer[0..16].store_le(self.control_word);
    }
}

/// DRV Outputs Modes of operation (0x7010:03 Ch.1 / 0x7020:03 Ch.2)
///
/// CSP = 8, CSV = 9, CST = 10, CSTCA = 11, DMC = 131.
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 8)]
pub struct DrvModesOfOperation {
    pub mode: u8,
}

impl RxPdoObject for DrvModesOfOperation {
    fn write(&self, buffer: &mut BitSlice<u8, Lsb0>) {
        buffer[0..8].store_le(self.mode);
    }
}

/// DRV Outputs Target position (0x7010:05 Ch.1 / 0x7020:05 Ch.2)
///
/// Unsigned 32-bit target position that wraps around modulo 2^32.
#[derive(Debug, Clone, Copy, Default, PdoObject, PartialEq, Eq)]
#[pdo_object(bits = 32)]
pub struct DrvTargetPosition {
    pub target_position: u32,
}

impl RxPdoObject for DrvTargetPosition {
    fn write(&self, buffer: &mut BitSlice<u8, Lsb0>) {
        buffer[0..32].store_le(self.target_position);
    }
}

/// All possible TxPDO maps for the EL7062.
///
/// Fields are declared in signal order; the `TxPdo` derive uses this order for
/// both the 0x1C13 PDO assignment and the frame bit-offsets.
#[derive(Debug, Clone, TxPdo)]
pub struct EL7062TxPdo {
    #[pdo_object_index(0x1A00)]
    pub fb_position_ch1: Option<FbPosition>,

    #[pdo_object_index(0x1A01)]
    pub status_word_ch1: Option<DrvStatusWord>,

    #[pdo_object_index(0x1A0E)]
    pub mode_of_operation_display_ch1: Option<DrvModeOfOperationDisplay>,

    #[pdo_object_index(0x1A06)]
    pub following_error_ch1: Option<DrvFollowingError>,

    #[pdo_object_index(0x1A80)]
    pub fb_position_ch2: Option<FbPosition>,

    #[pdo_object_index(0x1A81)]
    pub status_word_ch2: Option<DrvStatusWord>,

    #[pdo_object_index(0x1A8E)]
    pub mode_of_operation_display_ch2: Option<DrvModeOfOperationDisplay>,

    #[pdo_object_index(0x1A86)]
    pub following_error_ch2: Option<DrvFollowingError>,
}

/// All possible RxPDO maps for the EL7062.
///
/// Fields are declared in signal order; the `RxPdo` derive uses this order for
/// both the 0x1C12 PDO assignment and the frame bit-offsets.
#[derive(Debug, Clone, RxPdo)]
pub struct EL7062RxPdo {
    #[pdo_object_index(0x1600)]
    pub control_word_ch1: Option<DrvControlWord>,

    #[pdo_object_index(0x1608)]
    pub modes_of_operation_ch1: Option<DrvModesOfOperation>,

    #[pdo_object_index(0x1606)]
    pub target_position_ch1: Option<DrvTargetPosition>,

    #[pdo_object_index(0x1680)]
    pub control_word_ch2: Option<DrvControlWord>,

    #[pdo_object_index(0x1688)]
    pub modes_of_operation_ch2: Option<DrvModesOfOperation>,

    #[pdo_object_index(0x1686)]
    pub target_position_ch2: Option<DrvTargetPosition>,
}

pub const CSP_MODE: u8 = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EL7062PredefinedPdoAssignment {
    /// Rx: controlword + target position, per channel.
    /// Tx: position + statusword, per channel.
    Csp,
    /// `Csp` plus mode of operation RxPDO and mode display TxPDO.
    CspWithMode,
    /// `Csp` plus following error TxPDO.
    CspWithFollowingError,
    /// All CSP PDOs: mode + following error.
    #[default]
    CspWithModeAndFollowingError,
}

impl EL7062PredefinedPdoAssignment {
    /// Whether the mode of operation PDOs are part of the assignment.
    pub fn has_mode(self) -> bool {
        matches!(self, Self::CspWithMode | Self::CspWithModeAndFollowingError)
    }

    /// Whether the following error PDOs are part of the assignment.
    pub fn has_following_error(self) -> bool {
        matches!(
            self,
            Self::CspWithFollowingError | Self::CspWithModeAndFollowingError
        )
    }
}

impl PredefinedPdoAssignment<EL7062TxPdo, EL7062RxPdo> for EL7062PredefinedPdoAssignment {
    fn txpdo_assignment(&self) -> EL7062TxPdo {
        let with_mode = self.has_mode();
        let with_following_error = self.has_following_error();

        EL7062TxPdo {
            fb_position_ch1: Some(FbPosition::default()),
            status_word_ch1: Some(DrvStatusWord::default()),
            mode_of_operation_display_ch1: with_mode.then(DrvModeOfOperationDisplay::default),
            following_error_ch1: with_following_error.then(DrvFollowingError::default),
            fb_position_ch2: Some(FbPosition::default()),
            status_word_ch2: Some(DrvStatusWord::default()),
            mode_of_operation_display_ch2: with_mode.then(DrvModeOfOperationDisplay::default),
            following_error_ch2: with_following_error.then(DrvFollowingError::default),
        }
    }

    fn rxpdo_assignment(&self) -> EL7062RxPdo {
        let with_mode = self.has_mode();

        EL7062RxPdo {
            control_word_ch1: Some(DrvControlWord::default()),
            modes_of_operation_ch1: with_mode.then(DrvModesOfOperation::default),
            target_position_ch1: Some(DrvTargetPosition::default()),
            control_word_ch2: Some(DrvControlWord::default()),
            modes_of_operation_ch2: with_mode.then(DrvModesOfOperation::default),
            target_position_ch2: Some(DrvTargetPosition::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdo::{RxPdo, TxPdo};

    #[test]
    fn txpdo_sizes() {
        let assignment = EL7062PredefinedPdoAssignment::Csp;
        let txpdo = assignment.txpdo_assignment();
        // position (32) + statusword (16) per channel = 96 bits
        assert_eq!(txpdo.size(), 96);

        let assignment = EL7062PredefinedPdoAssignment::CspWithFollowingError;
        let txpdo = assignment.txpdo_assignment();
        // position (32) + statusword (16) + following error (32) per channel = 160 bits
        assert_eq!(txpdo.size(), 160);
    }

    #[test]
    fn rxpdo_sizes() {
        let assignment = EL7062PredefinedPdoAssignment::Csp;
        let rxpdo = assignment.rxpdo_assignment();
        // controlword (16) + target position (32) per channel = 96 bits
        assert_eq!(rxpdo.size(), 96);

        let assignment = EL7062PredefinedPdoAssignment::CspWithMode;
        let rxpdo = assignment.rxpdo_assignment();
        // controlword (16) + mode (8) + target position (32) per channel = 112 bits
        assert_eq!(rxpdo.size(), 112);
    }

    #[test]
    fn round_trip() {
        use bitvec::bitvec;

        let mut rxpdo =
            EL7062PredefinedPdoAssignment::CspWithModeAndFollowingError.rxpdo_assignment();
        rxpdo.control_word_ch1 = Some(DrvControlWord {
            control_word: 0x000F,
        });
        rxpdo.modes_of_operation_ch1 = Some(DrvModesOfOperation { mode: CSP_MODE });
        rxpdo.target_position_ch1 = Some(DrvTargetPosition {
            target_position: 0xDEAD_BEEF,
        });

        let mut buffer = bitvec![u8, Lsb0; 0; rxpdo.size()];
        rxpdo.write(buffer.as_mut_bitslice()).unwrap();

        // control word (ch1, 16 bit)
        let cw: u16 = buffer[0..16].load_le();
        assert_eq!(cw, 0x000F);
        // mode (ch1, next 8 bit)
        let mode: u8 = buffer[16..24].load_le();
        assert_eq!(mode, CSP_MODE);
        // target position (ch1, next 32 bit)
        let pos: u32 = buffer[24..56].load_le();
        assert_eq!(pos, 0xDEAD_BEEF);
    }
}
