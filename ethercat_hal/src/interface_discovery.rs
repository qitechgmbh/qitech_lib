
use std::{ffi::CString, mem, os::fd::RawFd};

use libc::{getifaddrs, freeifaddrs, ifaddrs};
use std::ffi::CStr;
use std::ptr;

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

pub fn list_ethernet_interfaces() -> Result<Vec<Interface>,anyhow::Error>{
    let mut ifaddr: *mut ifaddrs = ptr::null_mut();
    // Safety: getifaddrs populates a linked list of interface structures.
    // We must ensure we free this memory using freeifaddrs later.
    unsafe {
        if getifaddrs(&mut ifaddr) == -1 {
            eprintln!("Error calling getifaddrs");
            return Err(anyhow::anyhow!("Error calling getifaddrs"));
        }
        let mut vec : Vec<Interface> = vec![];
        let mut curr = ifaddr;

        while !curr.is_null() {
            let interface = *curr;            
            let flags = interface.ifa_flags;
            // Convert the C string name to a Rust &str
            if !interface.ifa_name.is_null() {
                let name = CStr::to_string_lossy(CStr::from_ptr(interface.ifa_name)).into_owned(); 
					
				println!("Interface {} {}",name,flags & libc::IFF_LOOPBACK as u32);
				if (flags & libc::IFF_LOOPBACK as u32) == 1 {    	
				    curr = interface.ifa_next;	
				    println!("is Loopback");	
    				continue;
				}     

                // Determine address family (IPv4, IPv6, or Packet/Link)
                let interface = if !interface.ifa_addr.is_null() {
                    match (*interface.ifa_addr).sa_family as i32 {
                        libc::AF_INET => Interface {link_type: LinkType::Ipv4, name},
                        libc::AF_INET6 => Interface {link_type: LinkType::Ipv6, name},
                        libc::AF_PACKET => Interface {link_type: LinkType::Link, name}, 
                        _ => Interface {link_type: LinkType::Unknown, name},
                    }
                } else {
                    Interface {link_type: LinkType::Unknown, name}
                };
                vec.push(interface);
            }
            curr = interface.ifa_next;
        }
        // Clean up the memory allocated by getifaddrs
        freeifaddrs(ifaddr);
        Ok( vec )
    }
}

fn open_raw_socket_libc(iface: &str) -> Result<RawFd, anyhow::Error> {
    unsafe {
        // 1. Create the socket
        // AF_PACKET: Low-level packet interface
        // SOCK_RAW: We want the full ethernet frame
        // ETH_P_ALL: Catch all protocols (0x0003), but we'll filter with bind
        let protocol = (0x88a4u16).to_be() as i32; // EtherCAT EtherType
        let fd = libc::socket(libc::AF_PACKET, libc::SOCK_RAW, protocol);
        
        if fd < 0 {
            return Err(anyhow::anyhow!("Socket creation failed: {}", std::io::Error::last_os_error()));
        }

        // 2. Get the interface index
        let if_name = CString::new(iface).map_err(|_| anyhow::anyhow!("Invalid interface name"))?;
        let if_index = libc::if_nametoindex(if_name.as_ptr());
        if if_index == 0 {
            libc::close(fd);
            return Err(anyhow::anyhow!("Interface {} not found", iface));
        }

        // 3. Prepare the sockaddr_ll structure
        // This structure binds the socket to the physical device
        let mut addr: libc::sockaddr_ll = mem::zeroed();
        addr.sll_family = libc::AF_PACKET as u16;
        addr.sll_ifindex = if_index as i32;
        addr.sll_protocol = (0x88a4u16).to_be();

        // 4. Bind the socket to the interface
        let addr_ptr = &addr as *const libc::sockaddr_ll as *const libc::sockaddr;
        let addr_len = mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t;

        if libc::bind(fd, addr_ptr, addr_len) == -1 {
            let err = std::io::Error::last_os_error();
            libc::close(fd);
            return Err(anyhow::anyhow!("Bind failed: {}", err));
        }

        // 5. Set a receive timeout so discovery doesn't hang
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
        // Send the Wireshark bytes
        let sent = libc::send(fd, packet.as_ptr() as *const libc::c_void, packet.len(), 0);
        if sent < 0 {
            println!("Send failed");
            return false;
        }

        // Buffer for response
        let mut buf = [0u8; 1514];
        let received = libc::recv(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0);

        if received > 0 {
            println!("Received {} bytes on this interface", received);
            // Check WKC in the response...                        
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