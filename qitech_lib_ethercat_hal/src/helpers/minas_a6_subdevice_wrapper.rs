use crate::helpers::ethercrab_types::EthercrabSubDevicePreoperational;
use anyhow::{Error, anyhow};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct PdoMapping {
    pub object_index: u16,
    pub sub_index: u8,
    pub bit_length: u8,
}

impl PdoMapping {
    pub fn to_u32(self) -> u32 {
        ((self.object_index as u32) << 16)
            | ((self.sub_index as u32) << 8)
            | (self.bit_length as u32)
    }
}

pub struct EtherCATSlaveWrapper<'a> {
    device: &'a EthercrabSubDevicePreoperational<'a>,
}

impl<'a> EtherCATSlaveWrapper<'a> {
    pub fn new(device: &'a EthercrabSubDevicePreoperational<'a>) -> Self {
        Self { device }
    }

    // SDO write helpers

    pub async fn write_sdo_u8(&self, index: u16, subindex: u8, value: u8) -> Result<(), Error> {
        self.device
            .sdo_write(index, subindex, value)
            .await
            .map_err(|e| anyhow!("sdo_write u8  [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    pub async fn write_sdo_u16(&self, index: u16, subindex: u8, value: u16) -> Result<(), Error> {
        self.device
            .sdo_write(index, subindex, value)
            .await
            .map_err(|e| anyhow!("sdo_write u16 [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    pub async fn write_sdo_u32(&self, index: u16, subindex: u8, value: u32) -> Result<(), Error> {
        self.device
            .sdo_write(index, subindex, value)
            .await
            .map_err(|e| anyhow!("sdo_write u32 [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    pub async fn write_sdo_i16(&self, index: u16, subindex: u8, value: i16) -> Result<(), Error> {
        self.device
            .sdo_write(index, subindex, value)
            .await
            .map_err(|e| anyhow!("sdo_write i16 [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    pub async fn write_sdo_i32(&self, index: u16, subindex: u8, value: i32) -> Result<(), Error> {
        self.device
            .sdo_write(index, subindex, value)
            .await
            .map_err(|e| anyhow!("sdo_write i32 [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    // SDO read helpers
    pub async fn read_sdo_u16(&self, index: u16, subindex: u8) -> Result<u16, Error> {
        self.device
            .sdo_read::<u16>(index, subindex)
            .await
            .map_err(|e| anyhow!("sdo_read  u16 [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    pub async fn read_sdo_u32(&self, index: u16, subindex: u8) -> Result<u32, Error> {
        self.device
            .sdo_read::<u32>(index, subindex)
            .await
            .map_err(|e| anyhow!("sdo_read  u32 [{:#06x}/{:#04x}]: {:?}", index, subindex, e))
    }

    // PDO assignment / mapping

    pub async fn assign_pdos(&self, assign_index: u16, pdo_indices: &[u16]) -> Result<(), Error> {
        self.write_sdo_u8(assign_index, 0x00, 0).await?;
        for (i, &pdo_index) in pdo_indices.iter().enumerate() {
            self.write_sdo_u16(assign_index, (i + 1) as u8, pdo_index)
                .await?;
        }
        self.write_sdo_u8(assign_index, 0x00, pdo_indices.len() as u8)
            .await
    }

    pub async fn configure_pdo_mapping(
        &self,
        map_index: u16,
        mappings: &[PdoMapping],
    ) -> Result<(), Error> {
        self.write_sdo_u8(map_index, 0x00, 0).await?;
        for (i, mapping) in mappings.iter().enumerate() {
            self.write_sdo_u32(map_index, (i + 1) as u8, mapping.to_u32())
                .await?;
        }
        self.write_sdo_u8(map_index, 0x00, mappings.len() as u8)
            .await
    }
}

pub async fn wait_status<F>(
    mut input_fn: F,
    status_word_byte_offset: usize,
    expected_status: u16,
    status_description: &str,
    status_mask: u16,
    timeout: Duration,
) -> Result<(), Error>
where
    F: FnMut() -> Vec<u8>,
{
    let deadline = Instant::now() + timeout;

    loop {
        let raw = input_fn();
        let end = status_word_byte_offset + 2;

        if end <= raw.len() {
            let status = u16::from_le_bytes([
                raw[status_word_byte_offset],
                raw[status_word_byte_offset + 1],
            ]);
            if (status & status_mask) == expected_status {
                return Ok(());
            }
        }

        if Instant::now() >= deadline {
            return Err(anyhow!("Timeout waiting for {}", status_description));
        }

        smol::Timer::after(Duration::from_millis(2)).await;
    }
}
