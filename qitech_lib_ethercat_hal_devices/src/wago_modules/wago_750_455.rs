use crate::{EthercatDeviceIdentity, IdentifiedDevice};
use bitvec::field::BitField;
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use qitech_lib_ethercat_hal::{DeviceError, PdoError};
use qitech_lib_ethercat_hal_derive::PdoObject;

use crate::{
    DynamicEthercatDevice, EthercatDevice, EthercatDeviceProcessing, EthercatDynamicPDO,
    NewEthercatDevice, SubDeviceProductTuple,
};
use qitech_lib_ethercat_hal::io::analog_input::AnalogCurrentInputDevice;
use qitech_lib_ethercat_hal::pdo::TxPdoObject;
use qitech_lib_units::electric_current::milliampere;
use qitech_lib_units::f64::ElectricCurrent;

#[derive(Debug, Default, PdoObject)]
#[pdo_object(bits = 16)]
struct AnalogInput {
    pub value: f64,
    pub error: bool,
}

impl TxPdoObject for AnalogInput {
    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        let raw: u16 = bits.load_le();
        let value = raw >> 4;
        let max_value = 0x7ff;

        self.error = (raw & 0x0003) == 0x0003;
        self.value = value as f64 / max_value as f64;
    }
}

#[derive(Default)]
pub struct Wago750_455TxPdo {
    ai1: AnalogInput,
    ai2: AnalogInput,
    ai3: AnalogInput,
    ai4: AnalogInput,
}

pub struct Wago750_455 {
    tx_bit_offset: usize,
    rx_bit_offset: usize,
    tx_pdo: Wago750_455TxPdo,
}

impl AnalogCurrentInputDevice for Wago750_455 {
    fn get_minimum_current(&self) -> ElectricCurrent {
        ElectricCurrent::new::<milliampere>(4.0)
    }

    fn get_maximum_current(&self) -> ElectricCurrent {
        ElectricCurrent::new::<milliampere>(20.0)
    }

    fn get_current_relative(&self, port: usize) -> Result<f64, DeviceError> {
        let input = match port {
            0 => &self.tx_pdo.ai1,
            1 => &self.tx_pdo.ai2,
            2 => &self.tx_pdo.ai3,
            3 => &self.tx_pdo.ai4,
            _ => {
                return Err(DeviceError::InvalidPort {
                    device: "Wago750_455",
                    port,
                });
            }
        };

        if input.error {
            return Err(DeviceError::ChannelFault {
                device: "Wago750_455",
                channel: port,
                fault: "Wiring error or current outside of valid range",
            });
        }

        Ok(input.value)
    }

    fn get_port_count(&self) -> usize {
        4
    }
}

impl DynamicEthercatDevice for Wago750_455 {}

impl EthercatDynamicPDO for Wago750_455 {
    fn get_tx_offset(&self) -> usize {
        self.tx_bit_offset
    }

    fn get_rx_offset(&self) -> usize {
        self.rx_bit_offset
    }

    fn set_tx_offset(&mut self, offset: usize) {
        self.tx_bit_offset = offset
    }

    fn set_rx_offset(&mut self, offset: usize) {
        self.rx_bit_offset = offset
    }
}

impl EthercatDevice for Wago750_455 {
    fn into_any_boxed(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
    fn input(&mut self, input: &BitSlice<u8, Lsb0>) -> Result<(), PdoError> {
        self.tx_pdo.ai1.read(&input[0..16]);
        self.tx_pdo.ai2.read(&input[16..32]);
        self.tx_pdo.ai3.read(&input[32..48]);
        self.tx_pdo.ai4.read(&input[48..64]);

        Ok(())
    }

    fn input_len(&self) -> usize {
        64
    }

    fn output(
        &self,
        _output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), PdoError> {
        Ok(())
    }

    fn output_len(&self) -> usize {
        0
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

    fn input_checked(
        &mut self,
        input: &bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), PdoError> {
        self.input(input)
    }

    fn output_checked(
        &self,
        _output: &mut bitvec::prelude::BitSlice<u8, bitvec::prelude::Lsb0>,
    ) -> Result<(), PdoError> {
        Ok(())
    }
}

impl EthercatDeviceProcessing for Wago750_455 {}

impl NewEthercatDevice for Wago750_455 {
    fn new() -> Self {
        Self {
            tx_bit_offset: 0,
            rx_bit_offset: 0,
            tx_pdo: Wago750_455TxPdo::default(),
        }
    }
}

impl std::fmt::Debug for Wago750_455 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wago750_455")
    }
}

pub const WAGO_750_455_VENDOR_ID: u32 = 0x00000021;
pub const WAGO_750_455_PRODUCT_ID: u32 = 0x045541b3;
pub const WAGO_750_455_MODULE_IDENT: SubDeviceProductTuple =
    (WAGO_750_455_VENDOR_ID, WAGO_750_455_PRODUCT_ID);

impl IdentifiedDevice for Wago750_455 {
    const IDENTITY: EthercatDeviceIdentity = EthercatDeviceIdentity {
        // Modules have no vendor id of their own, it is the vendor of the 750-354 coupler
        vendor: WAGO_750_455_VENDOR_ID,
        product: WAGO_750_455_PRODUCT_ID,
        revisions: &[],
    };
}
