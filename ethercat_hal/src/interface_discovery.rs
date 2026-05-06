use std::{ffi::CString, mem, os::fd::RawFd};
use std::ffi::CStr;

#[derive(Debug)]
pub enum LinkType {
	Link,
	Unknown,
	Ipv4,
	Ipv6
}


#[derive(Debug)]
pub struct Interface {
	pub link_type : LinkType,
	pub name : String,
}

pub fn list_ethernet_interfaces() -> Result<Vec<Interface>, anyhow::Error> {
    let mut ifaddr: *mut libc::ifaddrs = std::ptr::null_mut();    
    unsafe {
        
        if libc::getifaddrs(&mut ifaddr) == -1 {
            return Err(anyhow::anyhow!("Error calling getifaddrs: {}", std::io::Error::last_os_error()));
        }

        let mut vec: Vec<Interface> = vec![];
        let mut curr = ifaddr;
        while !curr.is_null() {
            let ifa = *curr;             
            if !ifa.ifa_name.is_null() {
                let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();
                let flags = ifa.ifa_flags;               
                if (flags & libc::IFF_LOOPBACK as u32) != 0 {                    
                    curr = ifa.ifa_next;
                    continue;
                }

                if !ifa.ifa_addr.is_null() {
                    let family = (*ifa.ifa_addr).sa_family as i32;
                    
                    let link_type = match family {
                        libc::AF_INET => LinkType::Ipv4,
                        libc::AF_INET6 => LinkType::Ipv6,
                        libc::AF_PACKET => LinkType::Link,
                        _ => LinkType::Unknown,
                    };
                    vec.push(Interface { link_type, name });
                }
            }            
            // Always move to next before loop ends
            curr = ifa.ifa_next;
        }

        libc::freeifaddrs(ifaddr);
        Ok(vec)
    }
}

fn open_raw_socket_libc(iface: &str) -> Result<RawFd, anyhow::Error> {
    unsafe {
        let protocol = (0x88a4u16).to_be() as i32; // EtherCAT EtherType
        let fd = libc::socket(libc::AF_PACKET, libc::SOCK_RAW, protocol);
        
        if fd < 0 {
            return Err(anyhow::anyhow!("Socket creation failed: {}", std::io::Error::last_os_error()));
        }

        let if_name = CString::new(iface).map_err(|_| anyhow::anyhow!("Invalid interface name"))?;
        let if_index = libc::if_nametoindex(if_name.as_ptr());
        if if_index == 0 {
            libc::close(fd);
            return Err(anyhow::anyhow!("Interface {} not found", iface));
        }

        let mut addr: libc::sockaddr_ll = mem::zeroed();
        addr.sll_family = libc::AF_PACKET as u16;
        addr.sll_ifindex = if_index as i32;
        addr.sll_protocol = (0x88a4u16).to_be();

        let addr_ptr = &addr as *const libc::sockaddr_ll as *const libc::sockaddr;
        let addr_len = mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;

        if libc::bind(fd, addr_ptr, addr_len) == -1 {
            let err = std::io::Error::last_os_error();
            libc::close(fd);
            return Err(anyhow::anyhow!("Bind failed: {}", err));
        }

        let timeout = libc::timeval {
            tv_sec: 0,
            tv_usec: 1000, // 1ms
        };

        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            &timeout as *const _ as *const libc::c_void,
            mem::size_of::<libc::timeval>() as libc::socklen_t,
        );

        Ok(fd)
    }
}


fn test_discovery(fd: RawFd, packet: &[u8]) -> bool{
    unsafe {
        let sent = libc::send(fd, packet.as_ptr() as *const libc::c_void, packet.len(), 0);
        if sent < 0 {
            return false;
        }
        let mut buf = [0u8; 1514];
        let received = libc::recv(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0);
		
        if received > 0 && buf[12] == 0x88 && buf[13] == 0xA4 {
            return true;
        } else {
            println!("Timeout or no data");
            return false;
        }
    }
}



pub fn test_interface(interface_name : &str) -> Result<(),anyhow::Error> {
	const ETHERCAT_DISCOVERY_FRAME: [u8; 29] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1, 0x1, 0x1, 0x1, 0x1, 0x1, 0x88, 0xa4, 0xd, 0x10, 0x8, 0x1, 0x0, 0x0, 0x3, 0x1, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0];
	let fd = open_raw_socket_libc(interface_name)?;
	if test_discovery(fd,&ETHERCAT_DISCOVERY_FRAME) {
		Ok(())
	}else {
		Err( anyhow::anyhow!("Interface {:?} is not Ethercat",interface_name) )		
	}
}