use libc::{freeifaddrs, getifaddrs, ifaddrs};
use std::ffi::CStr;
use std::ptr;
#[cfg(target_os = "linux")]
use std::{ffi::CString, mem, os::fd::RawFd};

#[derive(Debug, Clone)]
pub enum LinkType {
    Link,
    Unknown,
    Ipv4,
    Ipv6,
}

#[derive(Debug, Clone)]
pub struct Interface {
    pub link_type: LinkType,
    pub name: String,
}

fn is_wired_ethernet_device(name: &str) -> bool {
    let lower = name.to_lowercase();
    // Virtual devices to exclude
    let is_virtual = lower.starts_with("gif")
        || lower.starts_with("bridge")
        || lower.starts_with("veth")
        || lower.starts_with("docker")
        || lower.starts_with("br-")
        || lower.starts_with("vlan")
        || lower.starts_with("lo")
        || lower.starts_with("sit")
        || lower.starts_with("gre")
        || lower.starts_with("virbr")
        || lower.starts_with("vmnet");

    // Physical devices to include: en, eth, bond (wired only)
    let is_physical =
        lower.starts_with("en") || lower.starts_with("eth") || lower.starts_with("bond");

    is_physical && !is_virtual
}

pub fn list_ethernet_interfaces() -> Result<Vec<Interface>, anyhow::Error> {
    let mut ifaddr: *mut ifaddrs = ptr::null_mut();
    // getifaddrs populates a linked list of interface structures.
    unsafe {
        if getifaddrs(&mut ifaddr) == -1 {
            // eprintln!("Error calling getifaddrs");
            return Err(anyhow::anyhow!("Error calling getifaddrs"));
        }
        let mut vec: Vec<Interface> = vec![];
        let mut curr = ifaddr;

        while !curr.is_null() {
            let interface = *curr;
            let flags = interface.ifa_flags;
            // Convert the C string name to a Rust &str
            if !interface.ifa_name.is_null() {
                let name = CStr::to_string_lossy(CStr::from_ptr(interface.ifa_name)).into_owned();
                if (flags & libc::IFF_LOOPBACK as u32) != 0 {
                    curr = interface.ifa_next;
                    continue;
                }
                let interface = if !interface.ifa_addr.is_null() {
                    match (*interface.ifa_addr).sa_family as i32 {
                        libc::AF_INET => Interface {
                            link_type: LinkType::Ipv4,
                            name,
                        },
                        libc::AF_INET6 => Interface {
                            link_type: LinkType::Ipv6,
                            name,
                        },
                        #[cfg(target_os = "linux")]
                        libc::AF_PACKET => Interface {
                            link_type: LinkType::Link,
                            name,
                        },
                        #[cfg(target_os = "macos")]
                        libc::AF_LINK => Interface {
                            link_type: LinkType::Link,
                            name,
                        },
                        _ => Interface {
                            link_type: LinkType::Unknown,
                            name,
                        },
                    }
                } else {
                    Interface {
                        link_type: LinkType::Unknown,
                        name,
                    }
                };
                vec.push(interface);
            }
            curr = interface.ifa_next;
        }
        freeifaddrs(ifaddr);
        // Deduplicate by name, keeping Link type entries when available
        let mut deduped: Vec<Interface> = vec![];
        for iface in vec {
            if let Some(existing) = deduped.iter_mut().find(|e| e.name == iface.name) {
                // Prefer Link type over others
                if matches!(iface.link_type, LinkType::Link)
                    && !matches!(existing.link_type, LinkType::Link)
                {
                    *existing = iface;
                }
            } else {
                deduped.push(iface);
            }
        }
        // Sort to prefer physical devices (en, eth, etc.) before virtual (gif, bridge, etc.)
        // Secondary sort by name in reverse (so en9 before en1, en6 before en0)
        deduped.sort_by(|a, b| {
            match is_wired_ethernet_device(&a.name)
                .cmp(&is_wired_ethernet_device(&b.name))
                .reverse()
            {
                std::cmp::Ordering::Equal => b.name.cmp(&a.name), // reverse name sort for secondary
                other => other,
            }
        });
        Ok(deduped)
    }
}

// RawFd is just a c_int (i32 basically)
#[cfg(target_os = "linux")]
fn open_raw_socket_libc(iface: &str) -> Result<RawFd, anyhow::Error> {
    unsafe {
        let protocol = (0x88a4u16).to_be() as i32; // EtherCAT EtherType
        let fd = libc::socket(libc::AF_PACKET, libc::SOCK_RAW, protocol);
        if fd < 0 {
            return Err(anyhow::anyhow!(
                "Socket creation failed: {}",
                std::io::Error::last_os_error()
            ));
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

#[cfg(target_os = "linux")]
fn test_discovery(fd: RawFd, packet: &[u8]) -> bool {
    unsafe {
        let sent = libc::send(fd, packet.as_ptr() as *const libc::c_void, packet.len(), 0);
        if sent < 0 {
            return false;
        }

        // Buffer for response
        let mut buf = [0u8; 1514];
        let received = libc::recv(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0);
        return received > 0 && buf[12] == 0x88 && buf[13] == 0xa4;
    }
}

#[cfg(target_os = "linux")]
pub fn test_interface(interface_name: &str) -> Result<(), anyhow::Error> {
    const ETHERCAT_DISCOVERY_FRAME: [u8; 29] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x1, 0x1, 0x1, 0x1, 0x1, 0x1, 0x88, 0xa4, 0xd, 0x10,
        0x8, 0x1, 0x0, 0x0, 0x3, 0x1, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    ];
    let fd = open_raw_socket_libc(interface_name)?;
    let result = if test_discovery(fd, &ETHERCAT_DISCOVERY_FRAME) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Interface {:?} is not Ethercat",
            interface_name
        ))
    };
    unsafe { libc::close(fd) };
    result
}

// ── macOS BPF (Berkeley Packet Filter) ────────────────────────────────
// Native raw-packet I/O via /dev/bpfN.  No ethercrab / AF_PACKET required.

#[cfg(target_os = "macos")]
use std::{ffi::CString, mem, os::fd::RawFd};

#[cfg(target_os = "macos")]
fn open_bpf(interface_name: &str) -> Result<RawFd, anyhow::Error> {
    unsafe {
        for i in 0..16 {
            let dev = CString::new(format!("/dev/bpf{i}")).unwrap();
            let fd = libc::open(dev.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK);
            if fd < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EBUSY) {
                    continue;
                }
                return Err(anyhow::anyhow!("Failed to open /dev/bpf{i}"));
            }

            let one: libc::c_uint = 1;
            if libc::ioctl(fd, libc::BIOCIMMEDIATE, &one) == -1 {
                libc::close(fd);
                return Err(anyhow::anyhow!("BIOCIMMEDIATE failed"));
            }

            let mut ifr: libc::ifreq = mem::zeroed();
            for (dst, &src) in ifr.ifr_name.iter_mut().zip(interface_name.as_bytes()) {
                *dst = src as libc::c_char;
            }

            if libc::ioctl(fd, libc::BIOCSETIF, &ifr) == -1 {
                libc::close(fd);
                return Err(anyhow::anyhow!("BIOCSETIF({interface_name}) failed"));
            }

            return Ok(fd);
        }
        Err(anyhow::anyhow!("No free BPF device"))
    }
}

#[cfg(target_os = "macos")]
fn contains_ethercat_frame(data: &[u8]) -> bool {
    let mut pos = 0;
    while pos + 18 <= data.len() {
        let hdrlen = u16::from_le_bytes([data[pos + 16], data[pos + 17]]) as usize;
        let caplen = u32::from_le_bytes([
            data[pos + 12],
            data[pos + 13],
            data[pos + 14],
            data[pos + 15],
        ]) as usize;
        if hdrlen < 18 || pos + hdrlen + caplen > data.len() {
            break;
        }
        let pkt = &data[pos + hdrlen..pos + hdrlen + caplen];
        if pkt.len() >= 14 && pkt[12] == 0x88 && pkt[13] == 0xa4 {
            return true;
        }
        pos += (hdrlen + caplen + 3) & !3;
    }
    false
}

#[cfg(target_os = "macos")]
pub fn test_interface(interface_name: &str) -> Result<(), anyhow::Error> {
    const FRAME: [u8; 60] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x88, 0xa4, 0x0d,
        0x10, 0x08, 0x01, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let fd = open_bpf(interface_name)?;
    let result = probe_ethercat(fd, interface_name, &FRAME);
    unsafe {
        libc::close(fd);
    }
    result
}

#[cfg(target_os = "macos")]
fn probe_ethercat(fd: RawFd, interface_name: &str, frame: &[u8]) -> Result<(), anyhow::Error> {
    unsafe {
        let mut buf = [0u8; 4096];
        while libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) > 0 {}

        let n = libc::write(fd, frame.as_ptr() as *const libc::c_void, frame.len());
        if n != frame.len() as isize {
            return Err(anyhow::anyhow!("BPF write: sent {n}/{}", frame.len()));
        }

        let start = std::time::Instant::now();
        loop {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n > 0 && contains_ethercat_frame(&buf[..n as usize]) {
                return Ok(());
            }
            if start.elapsed() >= std::time::Duration::from_secs(2) {
                return Err(anyhow::anyhow!("No EtherCAT response on {interface_name}"));
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

// ── Other platforms (neither Linux nor macOS) ─────────────────────────
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn test_interface(interface_name: &str) -> Result<(), anyhow::Error> {
    Err(anyhow::anyhow!(
        "EtherCAT interface discovery is not available on this platform (interface: {})",
        interface_name
    ))
}
