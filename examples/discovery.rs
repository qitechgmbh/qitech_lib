use qitech_lib_ethercat_hal::interface_discovery::{list_ethernet_interfaces, test_interface};
fn main() {
    let t = list_ethernet_interfaces();
    match t {
        Ok(interfaces) => {
            for interface in interfaces {
                match interface.link_type {
                    qitech_lib_ethercat_hal::interface_discovery::LinkType::Link => (),
                    qitech_lib_ethercat_hal::interface_discovery::LinkType::Unknown => continue,
                    qitech_lib_ethercat_hal::interface_discovery::LinkType::Ipv4 => continue,
                    qitech_lib_ethercat_hal::interface_discovery::LinkType::Ipv6 => continue,
                };

                let res = test_interface(&interface.name);
                match res {
                    Ok(_) => println!("{} is ethercat", interface.name),
                    Err(_) => println!("{} is not ethercat", interface.name),
                }
            }
        }
        Err(_) => {
            println!("no interfaces found");
        }
    }
}
