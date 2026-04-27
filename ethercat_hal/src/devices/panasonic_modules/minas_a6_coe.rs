use crate::EncoderResolution;
use crate::devices::panasonic_modules::minas_a6::MotorHomingConfig;
use crate::{
    ChannelRequest, ChannelRequests, ChannelResponse, EtherCATThreadChannel,
    EtherCATThreadResponseChannel,
    coe::{ConfigurableDevice, Configuration},
    devices::panasonic_modules::minas_a6::MinasA6BMotor,
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

impl MinasA6BConfiguration {
    pub fn write_config_and_get_resolution(
        &self,
        ecat_channel: EtherCATThreadChannel,
        device_address: u16,
    ) -> Result<EncoderResolution, anyhow::Error> {
        let (response_tx, response_rx) = std::sync::mpsc::channel();
        ecat_channel.0.send(ChannelRequest {
            channel_request: ChannelRequests::ConfigureMinasA6B {
                device_address,
                homing_config: self.homing_config.clone(),
            },
            response_channel: EtherCATThreadResponseChannel(response_tx),
        })?;
        match response_rx.recv()? {
            ChannelResponse::ConfigureMinasA6BResponse(result) => result,
            _ => Err(anyhow::anyhow!("Unexpected response from EtherCAT thread")),
        }
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
