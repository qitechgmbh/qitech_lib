use qitech_lib_ethercat_hal::{MetaSubdevice, device::Module};

/// Expects coupler_start_tx and rx in bits!
pub fn convert_module_to_meta_ethercat_device(module : &Module, coupler_start_tx : usize, coupler_start_rx : usize) -> MetaSubdevice  {
    let mut name : [u8;128] = [0u8;128];
    let b = module.name.clone().into_bytes();    
    name[..b.len()].copy_from_slice(b.as_slice());
    
    let start_tx = coupler_start_tx + module.tx_offset; 
    let end_tx = coupler_start_tx + module.tx_offset + module.tx_size;
    let start_rx = coupler_start_rx + module.rx_offset;
    let end_rx = coupler_start_rx + module.rx_offset + module.rx_size;

    MetaSubdevice { 
        name, 
        product_id: module.product_id,
        revision: 0x00, 
        vendor: module.vendor_id, 
        start_tx: start_tx / 8,
        end_tx: end_tx / 8,
        start_rx: start_rx / 8,
        end_rx: end_rx / 8, 
        device_address: module.slot, 
        initialized: true 
    }
}