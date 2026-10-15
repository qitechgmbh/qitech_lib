use super::{
    wago_750_402::WAGO_750_402_PRODUCT_ID,
    wago_750_430::WAGO_750_430_PRODUCT_ID,
    wago_750_455::WAGO_750_455_PRODUCT_ID,
    wago_750_501::WAGO_750_501_PRODUCT_ID,
    wago_750_530::WAGO_750_530_PRODUCT_ID,
    wago_750_531::WAGO_750_531_PRODUCT_ID,
    wago_750_554::WAGO_750_554_PRODUCT_ID,
    wago_750_652::WAGO_750_652_PRODUCT_ID,
    wago_750_671::WAGO_750_671_PRODUCT_ID,
    wago_750_672::WAGO_750_672_PRODUCT_ID,
    wago_750_1506::WAGO_750_1506_PRODUCT_ID,
};
use qitech_lib_ethercat_hal::{
    EtherCATThreadChannel, MetaSubdevice, device::Module, sdo::{RX_PDO_ASSIGNMENT_REG, TX_PDO_ASSIGNMENT_REG},
};
pub const MODULE_COUNT_INDEX: (u16, u8) = (0xf050, 0x00);

#[derive(Clone, Debug)]
pub struct ModulePdoMapping {
    pub offset: usize,    
    pub sub_index: usize,
    pub size:  usize,
    pub module_i: u32,
}

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

/// Call after all modules have been added
pub fn init_slot_modules(
    modules: &mut Vec<Module>,
    ecat_channel: EtherCATThreadChannel,
    device_address: u16,
) -> Result<(), qitech_lib_ethercat_hal::Error> {
    let tx_pdo_mappings = get_module_pdo_offsets(device_address, ecat_channel.clone(), true)?;
    let rx_pdo_mappings = get_module_pdo_offsets(device_address, ecat_channel.clone(), false)?;
    for m in modules.iter_mut() {
        m.tx_size = 0;
        for pdo_map in tx_pdo_mappings.iter() {
            m.tx_size += pdo_map.size;
        }

        m.rx_size = 0;
        for pdo_map in rx_pdo_mappings.iter() {
            m.rx_size += pdo_map.size;
        }

        let tx_pdo_mapping = tx_pdo_mappings
            .iter()
            .find(|map| map.module_i == m.slot.into() && map.sub_index == 0x1);

        if m.has_tx {
            m.tx_offset = match tx_pdo_mapping {
                Some(map) => map.offset,
                None => 0,
            }
        }

        let rx_pdo_mapping = rx_pdo_mappings
            .iter()
            .find(|map| map.module_i == m.slot.into() && map.sub_index == 0x1);
        if m.has_rx {
            m.rx_offset = match rx_pdo_mapping {
                Some(map) => map.offset,
                None => 0,
            }
        }
    }
    Ok(())
}


pub fn get_module_count(
    ecat_channel: EtherCATThreadChannel,
    device_address: u16,
) -> Result<usize, qitech_lib_ethercat_hal::Error> {
    match ecat_channel.sdo_read::<u8>(device_address, MODULE_COUNT_INDEX.0, MODULE_COUNT_INDEX.1) {
        Ok(value) => Ok(value as usize),
        Err(e) => Err(e.into()),
    }
}

pub fn calculate_module_index(pdo_mapping: u32, is_tx: bool) -> u32 {
    let start_index = match is_tx {
        true => 0x6000 as u32,
        false => 0x7000 as u32,
    };
    println!("{}",pdo_mapping);
    let index_in_hex = ((pdo_mapping & 0xFFFF0000) >> 16) - start_index;
    if index_in_hex < 16 {
        return 0;
    } else {
        return index_in_hex / 16;
    }
}

pub fn get_module_pdo_offsets(
    device_address: u16,
    ecat_channel: EtherCATThreadChannel,
    get_tx: bool,
) -> Result<Vec<ModulePdoMapping>, qitech_lib_ethercat_hal::Error> {
    let mut vec: Vec<ModulePdoMapping> = vec![];
    let mut bit_offset = 0;
    let start_subindex = 0x2;
    let index = match get_tx {
        true => (TX_PDO_ASSIGNMENT_REG, 0x00),
        false => (RX_PDO_ASSIGNMENT_REG, 0x00),
    };
    
    let start_pdo_index = match get_tx {
        true => 0x1a00 as u16,
        false => 0x1600 as u16,
    };

    let start_index = match get_tx {
        true => 0x6000 as u16,
        false => 0x7000 as u16,
    };

    let count_mappings = ecat_channel.sdo_read::<u8>(device_address, index.0, index.1)?;
    let pdo_index = ecat_channel.sdo_read::<u16>(device_address, index.0, 1)?;
    let pdo_map_count = ecat_channel.sdo_read::<u8>(device_address, pdo_index, 0)?;

    for i in 0..pdo_map_count {
        let pdo_mapping: u32 = ecat_channel.sdo_read(device_address, pdo_index, 1 + i)?;
        let bit_length = (pdo_mapping & 0xFF) as u8;
        bit_offset += bit_length as usize;
    }

    let mut mappings_without_coupler: Vec<(u16,u32)> = vec![];
    
    for i in start_subindex..=count_mappings {
        let pdo_index = ecat_channel.sdo_read(device_address, index.0, i)?;
        let pdo_map_count = ecat_channel.sdo_read::<u8>(device_address, pdo_index, 0)?;
        println!("count pdo_index {:X} {:X}", pdo_index,pdo_map_count);
        for j in 0..pdo_map_count {
            let pdo_mapping: u32 = ecat_channel.sdo_read(device_address, pdo_index, 1 + j)?;
            // when pdo_index - start >= 256 then we are handling a "Gap"
            // Commonly added after digital inputs/ouputs to "align" to boundaries
            if  (pdo_index - start_pdo_index) >= 256 {
                let pdo_mapping =  (start_index+256) as u32;
                println!("HELP adjusted pdomapping pdo_index sub_index {:X} {:X} {:X}", pdo_mapping,pdo_index, 1 + j);
            }
            println!("pdomapping pdo_index sub_index {:X} {:X} {:X}", pdo_mapping,pdo_index, 1 + j);
            mappings_without_coupler.push( (pdo_index,pdo_mapping) );
        }
    }
    println!("{:?}",&mappings_without_coupler);
    //mappings_without_coupler.sort();
    
    for (pdo_index, pdo_mapping) in mappings_without_coupler {
        // if distance is < 256 then its a normal mapping
        // Otherwise its a Gap
        if (pdo_index - start_pdo_index) < 256 && pdo_mapping > start_index as u32 {
            let module_i = calculate_module_index(pdo_mapping, get_tx);
            let bit_length = (pdo_mapping & 0xFF) as usize;
            let sub_index =  ((pdo_mapping & 0x0000FF00 ) >> 8) as usize;
            if module_i < 64 {
                vec.push(ModulePdoMapping {
                    offset: bit_offset,
                    sub_index,
                    size: bit_length,
                    module_i,
                });
            }
            bit_offset += bit_length;
        } else{            
            bit_offset += pdo_mapping as usize;
        };
    }
    //vec.sort_by_key(|e| (e.module_i, e.offset));
    // deduplicate by module_i, so we only have the offset to the start of inputs/outputs
    //vec.dedup_by(|a, b| a.module_i == b.module_i);
    Ok(vec)
}

/// vendor_id is set on all modules
/// device_address should be used to address the coupler that has the modules in its pdu
/// Or the "Box" whatever it is called for you
pub fn get_modules(
    ecat_channel: EtherCATThreadChannel,
    device_address: u16,
    vendor_id: u32,
) -> Result<Vec<crate::Module>, qitech_lib_ethercat_hal::Error> {
    const MODULES_START_ADDR: u16 = 0x9000;
    const MODULE_IDENT_SUBINDEX: u8 = 0x0a;
    let mut modules: Vec<Module> = vec![];
    let module_count = get_module_count(ecat_channel.clone(), device_address)?;

    for i in 0..module_count {
        let module_addr = MODULES_START_ADDR + (i * 0x10) as u16;
        let ident_iom =
            ecat_channel.sdo_read::<u32>(device_address, module_addr, MODULE_IDENT_SUBINDEX)?;
        // For Wago the IOM well be the product ID
        let mut module = Module {
            slot: i as u16,
            belongs_to_addr: device_address,
            vendor_id,
            product_id: ident_iom,
            has_tx: false,
            has_rx: false,
            tx_offset: 0,
            rx_offset: 0,
            name: "".to_string(),
            tx_size: 0,
            rx_size: 0,
        };

        match ident_iom {
            // Add Module idents here:
            WAGO_750_1506_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = true;
                module.name = "750-1506".to_string();
            }
            WAGO_750_455_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = false;
                module.name = "750-455".to_string();
            }
            WAGO_750_501_PRODUCT_ID => {
                module.has_tx = false;
                module.has_rx = true;
                module.name = "750-501".to_string();
            }
            WAGO_750_530_PRODUCT_ID => {
                module.has_tx = false;
                module.has_rx = true;
                module.name = "750-530".to_string();
            }
            WAGO_750_531_PRODUCT_ID => {
                module.has_tx = false;
                module.has_rx = true;
                module.name = "750-531".to_string();
            }
            WAGO_750_652_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = true;
                module.name = "750-652".to_string();
            }
            WAGO_750_554_PRODUCT_ID => {
                module.has_tx = false;
                module.has_rx = true;
                module.name = "750-554".to_string();
            }
            WAGO_750_402_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = false;
                module.name = "750-402".to_string();
            }
            WAGO_750_430_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = false;
                module.name = "750-430".to_string();
            }
            WAGO_750_671_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = true;
                module.name = "750-671".to_string();
            }
            WAGO_750_672_PRODUCT_ID => {
                module.has_tx = true;
                module.has_rx = true;
                module.name = "750-672".to_string();
            }
            _ => tracing::warn!("found unknown/unimplemented module: {ident_iom}"),
        }
        modules.push(module);
    }
    init_slot_modules(&mut modules,ecat_channel,device_address)?;
    Ok(modules)
}
