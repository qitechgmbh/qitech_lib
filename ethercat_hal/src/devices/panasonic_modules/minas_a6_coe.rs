use crate::EncoderResolution;
use crate::devices::panasonic_modules::minas_a6::{
    GET_DATA_MAPPING, MinasA6BMotor, MotorHomingConfig, Reg, SET_DATA_MAPPING,
};
use crate::helpers::minas_a6_subdevice_wrapper::PdoMapping;
use crate::{
    EtherCATThreadChannel,
    coe::{ConfigurableDevice, Configuration},
};

#[derive(Debug, Clone)]
pub struct MinasA6BConfiguration {
    pub homing_config: MotorHomingConfig,
}

impl Default for MinasA6BConfiguration {
    fn default() -> Self {
        Self {
            homing_config: MotorHomingConfig::default(),
        }
    }
}

fn write_pdo_mapping(
    ch: &EtherCATThreadChannel,
    addr: u16,
    pdo_index: u16,
    entries: &[PdoMapping],
) -> Result<(), anyhow::Error> {
    // 1. Clear mapping count
    ch.sdo_write(addr, pdo_index, 0, 0u8)?;
    // 2. Write each mapping entry
    for (i, mapping) in entries.iter().enumerate() {
        ch.sdo_write(addr, pdo_index, (i + 1) as u8, mapping.to_u32())?;
    }
    // 3. Set mapping count
    ch.sdo_write(addr, pdo_index, 0, entries.len() as u8)?;
    Ok(())
}

/// Assign PDO mapping objects to a sync-manager (e.g. 0x1C12 / 0x1C13).
fn assign_pdos(
    ch: &EtherCATThreadChannel,
    addr: u16,
    assign_reg: u16,
    pdo_indices: &[u16],
) -> Result<(), anyhow::Error> {
    ch.sdo_write(addr, assign_reg, 0, 0u8)?;
    for (i, &pdo) in pdo_indices.iter().enumerate() {
        ch.sdo_write(addr, assign_reg, (i + 1) as u8, pdo)?;
    }
    ch.sdo_write(addr, assign_reg, 0, pdo_indices.len() as u8)?;
    Ok(())
}

fn configure_drive(ch: &EtherCATThreadChannel, addr: u16) -> Result<(), anyhow::Error> {
    // PDO mapping
    write_pdo_mapping(ch, addr, Reg::RX_PDO, &SET_DATA_MAPPING)?;
    write_pdo_mapping(ch, addr, Reg::TX_PDO, &GET_DATA_MAPPING)?;
    assign_pdos(ch, addr, Reg::RX_PDO_ASSIGN_ADDRESS, &[Reg::RX_PDO])?;
    assign_pdos(ch, addr, Reg::TX_PDO_ASSIGN_ADDRESS, &[Reg::TX_PDO])?;

    // Drive parameters
    ch.sdo_write(addr, Reg::POSITIONING_OPTION, 0, 0u16)?;
    ch.sdo_write(addr, Reg::POSITION_WINDOW, 0, 10_000u32)?;
    ch.sdo_write(addr, Reg::POSITION_WINDOW_TIME, 0, 0u16)?;
    ch.sdo_write(addr, Reg::FOLLOWING_WINDOW, 0, 100u32)?;
    ch.sdo_write(addr, Reg::FOLLOWING_WINDOW_TIME, 0, 1u16)?;
    ch.sdo_write(addr, Reg::ENCODER_SETTING, 0, 0i16)?; // absolute encoder
    ch.sdo_write(addr, Reg::MAX_TORQUE, 0, 500u16)?; // 50.0 %
    Ok(())
}

fn read_encoder_resolution(
    ch: &EtherCATThreadChannel,
    addr: u16,
) -> Result<EncoderResolution, anyhow::Error> {
    let increments: u32 = ch.sdo_read(addr, Reg::ENCODER_RESOLUTION, 1)?;
    let revolutions: u32 = ch.sdo_read(addr, Reg::ENCODER_RESOLUTION, 2)?;
    if revolutions == 0 {
        return Err(anyhow::anyhow!(
            "ENCODER_RESOLUTION sub-index 2 returned 0 revolutions"
        ));
    }
    Ok(EncoderResolution {
        increments,
        revolutions,
    })
}

fn setup_homing(
    ch: &EtherCATThreadChannel,
    addr: u16,
    enc: &EncoderResolution,
    homing: &MotorHomingConfig,
) -> Result<(), anyhow::Error> {
    ch.sdo_write(addr, Reg::HOMING_MODE, 0, homing.homing_direction.mode_code())?;
    ch.sdo_write(
        addr,
        Reg::HOMING_SPEED,
        1,
        enc.rps_to_inc_per_sec(homing.high_speed_rps),
    )?;
    ch.sdo_write(
        addr,
        Reg::HOMING_SPEED,
        2,
        enc.rps_to_inc_per_sec(homing.slow_speed_rps),
    )?;
    ch.sdo_write(
        addr,
        Reg::HOMING_ACC,
        0,
        enc.rps_to_inc_per_sec(homing.acceleration_rps_squared),
    )?;
    ch.sdo_write(addr, Reg::HOMING_OFFSET, 0, 0i32)?;
    Ok(())
}

impl MinasA6BConfiguration {
    pub fn write_config_and_get_resolution(
        &self,
        ch: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<EncoderResolution, anyhow::Error> {
        configure_drive(&ch, device_address)?;
        let enc = read_encoder_resolution(&ch, device_address)?;
        setup_homing(&ch, device_address, &enc, &self.homing_config)?;
        Ok(enc)
    }
}

impl Configuration for MinasA6BConfiguration {
    fn write_config(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<(), anyhow::Error> {
        self.write_config_and_get_resolution(ecat_channel, device_address)
            .map(|_| ())
    }
}

impl ConfigurableDevice<MinasA6BConfiguration> for MinasA6BMotor {
    fn write_config(
        &mut self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
        config: &MinasA6BConfiguration,
    ) -> Result<(), anyhow::Error> {
        let encoder_resolution =
            config.write_config_and_get_resolution(ecat_channel, device_address)?;

        self.homing_config = config.homing_config.clone();
        self.set_encoder_resolution(encoder_resolution);
        Ok(())
    }

    fn get_config(&self) -> MinasA6BConfiguration {
        MinasA6BConfiguration {
            homing_config: self.homing_config.clone(),
        }
    }
}
