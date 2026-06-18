use super::{EthercatDeviceProcessing, NewEthercatDevice, SubDeviceIdentityTuple};
use crate::io::analog_output::{AnalogOutputDevice, AnalogOutputOutput};
use crate::pdo::oversampling::{AnalogOutputOversample, CycleCount};
use crate::pdo::{RxPdo, TxPdo};
use ethercat_hal_derive::{EthercatDevice, RxPdo, TxPdo};

/// EL4732 2-channel analog output device with oversampling support
///
/// 16-bit resolution, -10V to +10V, E-Bus current: 180mA
///
/// `oversample_factor` must be one of: 1, 2, 3, 4, 5, 8, 10, 16, 20, 25, 32, 40, 50, 100
/// (as defined in the EL4732 ESI DC OpModes)
///
/// PDI layout per cycle:
///   SM0 (0x1000): ch1_cycle_count (u16) + [i16; N] = 2 + N*2 bytes
///   SM1 (0x1400): ch2_cycle_count (u16) + [i16; N] = 2 + N*2 bytes
///
/// DC sync is required for oversampling (AssignActivate = 0x0730).
/// For N=1: use DcSync::Sync0
/// For N>1: use DcSync::Sync01 { sync1_period: cycle_time / N }
#[derive(EthercatDevice)]
pub struct EL4732 {
    pub rxpdo: EL4732RxPdo,
    pub txpdo: EL4732TxPdo,
    is_used: bool,
    pub configuration: EL4732Configuration,
}

impl EthercatDeviceProcessing for EL4732 {}

impl std::fmt::Debug for EL4732 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EL4732 (OSFac={})",
            self.rxpdo
                .ch1_samples
                .as_ref()
                .map(|s| s.oversample_factor())
                .unwrap_or(1)
        )
    }
}

impl EL4732 {
    /// Create an EL4732 with a specific oversampling factor.
    ///
    /// `oversample_factor` must be one of:
    ///   1, 2, 3, 4, 5, 8, 10, 16, 20, 25, 32, 40, 50, 100
    pub fn new_with_oversample(oversample_factor: usize) -> Self {
        let configuration = EL4732Configuration::default();
        Self {
            rxpdo: EL4732RxPdo::new(oversample_factor),
            txpdo: EL4732TxPdo::default(),
            is_used: false,
            configuration,
        }
    }

    /// Write individual oversampled values for a channel.
    ///
    /// Use this for waveform generation where each slot within a cycle
    /// needs a different value (e.g. a sine wave). Values are in -1.0..=1.0.
    ///
    /// Panics if `samples.len()` does not match the configured oversample factor.
    pub fn set_output_samples(&mut self, port: usize, samples: &[f32]) {
        match port {
            0 => {
                if let Some(ch) = self.rxpdo.ch1_samples.as_mut() {
                    assert_eq!(
                        samples.len(),
                        ch.oversample_factor(),
                        "samples.len() must match oversample factor"
                    );
                    for (slot, &v) in ch.samples.iter_mut().zip(samples.iter()) {
                        *slot = normalize_voltage(v);
                    }
                }
                if let Some(cc) = self.rxpdo.ch1_cycle_count.as_mut() {
                    cc.increment();
                }
            }
            1 => {
                if let Some(ch) = self.rxpdo.ch2_samples.as_mut() {
                    assert_eq!(
                        samples.len(),
                        ch.oversample_factor(),
                        "samples.len() must match oversample factor"
                    );
                    for (slot, &v) in ch.samples.iter_mut().zip(samples.iter()) {
                        *slot = normalize_voltage(v);
                    }
                }
                if let Some(cc) = self.rxpdo.ch2_cycle_count.as_mut() {
                    cc.increment();
                }
            }
            _ => {}
        }
    }
}

impl NewEthercatDevice for EL4732 {
    /// Creates an EL4732 with oversample factor 1 (no oversampling).
    /// Use `EL4732::new_with_oversample(N)` for oversampling.
    fn new() -> Self {
        Self::new_with_oversample(1)
    }
}

/// Scale -1.0..=1.0 to i16 (-10V..=+10V)
fn normalize_voltage(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

impl AnalogOutputDevice for EL4732 {
    /// Sets both channels to a constant value across all oversampling slots.
    /// For per-slot control use `set_output_samples()` instead.
    fn set_output(&mut self, port: usize, value: AnalogOutputOutput) {
        let raw = normalize_voltage(value.0);
        match port {
            0 => {
                if let Some(ch) = self.rxpdo.ch1_samples.as_mut() {
                    ch.samples.fill(raw);
                }
                if let Some(cc) = self.rxpdo.ch1_cycle_count.as_mut() {
                    cc.increment();
                }
            }
            1 => {
                if let Some(ch) = self.rxpdo.ch2_samples.as_mut() {
                    ch.samples.fill(raw);
                }
                if let Some(cc) = self.rxpdo.ch2_cycle_count.as_mut() {
                    cc.increment();
                }
            }
            _ => {}
        }
    }

    fn get_port_count(&self) -> usize {
        2
    }
}

#[derive(Debug, Clone, RxPdo)]
pub struct EL4732RxPdo {
    #[pdo_object_index(0x1680)]
    pub ch1_cycle_count: Option<CycleCount>,
    #[pdo_object_index(0x1600)]
    pub ch1_samples: Option<AnalogOutputOversample>,
    #[pdo_object_index(0x1780)]
    pub ch2_cycle_count: Option<CycleCount>,
    #[pdo_object_index(0x1700)]
    pub ch2_samples: Option<AnalogOutputOversample>,
}

impl EL4732RxPdo {
    pub fn new(oversample_factor: usize) -> Self {
        Self {
            ch1_cycle_count: Some(CycleCount::default()),
            ch1_samples: Some(AnalogOutputOversample::new(oversample_factor)),
            ch2_cycle_count: Some(CycleCount::default()),
            ch2_samples: Some(AnalogOutputOversample::new(oversample_factor)),
        }
    }
}

impl Default for EL4732RxPdo {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone, TxPdo)]
pub struct EL4732TxPdo {}

impl Default for EL4732TxPdo {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EL4732Port {
    AO1 = 0,
    AO2 = 1,
}

#[derive(Debug, Clone)]
pub struct EL4732Configuration {
    pub oversample_factor: usize,
}

impl Default for EL4732Configuration {
    fn default() -> Self {
        Self {
            oversample_factor: 1,
        }
    }
}

pub const EL4732_VENDOR_ID: u32 = 0x2;
pub const EL4732_PRODUCT_ID: u32 = 0x127C3052;

// Revisions 0x00000000 and 0x00000001 are marked HideType (legacy) in the ESI
// and are intentionally excluded.
pub const EL4732_REVISION_A: u32 = 0x00020000;
pub const EL4732_REVISION_B: u32 = 0x00030000;
pub const EL4732_REVISION_C: u32 = 0x00040000;

pub const EL4732_IDENTITY_A: SubDeviceIdentityTuple =
    (EL4732_VENDOR_ID, EL4732_PRODUCT_ID, EL4732_REVISION_A);
pub const EL4732_IDENTITY_B: SubDeviceIdentityTuple =
    (EL4732_VENDOR_ID, EL4732_PRODUCT_ID, EL4732_REVISION_B);
pub const EL4732_IDENTITY_C: SubDeviceIdentityTuple =
    (EL4732_VENDOR_ID, EL4732_PRODUCT_ID, EL4732_REVISION_C);
