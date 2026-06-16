use libc::{freeifaddrs, getifaddrs, ifaddrs};
use std::ffi::CStr;
use std::ptr;
#[cfg(target_os = "linux")]
use std::{ffi::CString, mem, os::fd::RawFd};
use tracing::{error, warn};

#[derive(Debug)]
pub enum LinkType {
    Link,
    Unknown,
    Ipv4,
    Ipv6,
}

#[derive(Debug)]
pub struct Interface {
    pub link_type: LinkType,
    pub name: String,
}

pub fn list_ethernet_interfaces() -> Result<Vec<Interface>, anyhow::Error> {
    let mut ifaddr: *mut ifaddrs = ptr::null_mut();
    unsafe {
        if getifaddrs(&mut ifaddr) == -1 {
            error!("Error calling getifaddrs");
            return Err(anyhow::anyhow!("Error calling getifaddrs"));
        }
        let mut vec: Vec<Interface> = vec![];
        let mut curr = ifaddr;

        while !curr.is_null() {
            let interface = *curr;
            let flags = interface.ifa_flags;
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
        Ok(vec)
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
    let name = interface_name.as_bytes();
    if name.len() >= libc::IFNAMSIZ {
        return Err(anyhow::anyhow!("Interface name too long: {interface_name}"));
    }
    unsafe {
        for i in 0..256 {
            let dev = CString::new(format!("/dev/bpf{i}")).unwrap();
            let fd = libc::open(dev.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK);
            if fd < 0 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error() {
                    // Device in use — try the next one.
                    Some(libc::EBUSY) => continue,
                    // Past the last existing BPF node — all devices busy.
                    Some(libc::ENOENT) => {
                        return Err(anyhow::anyhow!(
                            "No free BPF device (checked /dev/bpf0../dev/bpf{i})"
                        ));
                    }
                    // EPERM/EACCES etc. — retrying other nodes won't help.
                    _ => return Err(anyhow::anyhow!("Failed to open /dev/bpf{i}: {err}")),
                }
            }
            // Immediate mode — read returns as soon as a packet arrives.
            let immediate: libc::c_uint = 1;
            if libc::ioctl(fd, libc::BIOCIMMEDIATE, &immediate) == -1 {
                let err = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(anyhow::anyhow!(
                    "BIOCIMMEDIATE on /dev/bpf{i} failed: {err}"
                ));
            }

            let mut ifr: libc::ifreq = mem::zeroed();
            for (dst, &src) in ifr.ifr_name.iter_mut().zip(name) {
                *dst = src as libc::c_char;
            }

            if libc::ioctl(fd, libc::BIOCSETIF, &ifr) == -1 {
                let err = std::io::Error::last_os_error();
                libc::close(fd);
                return Err(anyhow::anyhow!(
                    "BIOCSETIF({interface_name}) on /dev/bpf{i} failed: {err}"
                ));
            }
            return Ok(fd);
        }
        Err(anyhow::anyhow!("No free BPF device"))
    }
}

/// Scans a BPF read buffer (which may hold several concatenated records)
/// for a captured frame with the EtherCAT EtherType (0x88a4).
#[cfg(target_os = "macos")]
fn contains_ethercat_frame(data: &[u8]) -> bool {
    let align = libc::BPF_ALIGNMENT as usize;
    let mut off = 0usize;
    while off + mem::size_of::<libc::bpf_hdr>() <= data.len() {
        let hdr =
            unsafe { std::ptr::read_unaligned(data.as_ptr().add(off) as *const libc::bpf_hdr) };
        let hdrlen = hdr.bh_hdrlen as usize;
        let caplen = hdr.bh_caplen as usize;
        if hdrlen < mem::size_of::<libc::bpf_hdr>() || off + hdrlen + caplen > data.len() {
            break;
        }
        let pkt = &data[off + hdrlen..off + hdrlen + caplen];
        if pkt.len() >= 14 && pkt[12] == 0x88 && pkt[13] == 0xa4 {
            return true;
        }
        off += (hdrlen + caplen + align - 1) & !(align - 1);
    }
    false
}

#[cfg(target_os = "macos")]
pub fn test_interface(interface_name: &str) -> Result<(), anyhow::Error> {
    // EtherCAT broadcast discovery frame padded to Ethernet minimum (60 bytes).
    const FRAME: [u8; 60] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02, 0x00, 0x00, 0x00, 0x00, 0x01, 0x88, 0xa4, 0x0d,
        0x10, 0x08, 0x01, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let fd = open_bpf(interface_name)?;
    let result = probe_ethercat(fd, interface_name, &FRAME);
    unsafe {
        libc::close(fd);
    }
    if let Err(ref e) = result {
        warn!(
            "Interface '{}' does not respond to EtherCAT discovery: {}",
            interface_name, e
        );
    }
    result
}

#[cfg(target_os = "macos")]
fn probe_ethercat(fd: RawFd, interface_name: &str, frame: &[u8]) -> Result<(), anyhow::Error> {
    unsafe {
        let mut buf = [0u8; 4096];

        // Drain stale packets (non-blocking)
        while libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) > 0 {}

        // Send discovery frame
        if libc::write(fd, frame.as_ptr() as *const libc::c_void, frame.len()) < 0 {
            error!(
                "BPF write on '{}': {}",
                interface_name,
                std::io::Error::last_os_error()
            );
            return Err(anyhow::anyhow!(
                "BPF write: {}",
                std::io::Error::last_os_error()
            ));
        }

        // Wait for response (500ms timeout, non-blocking read + nanosleep)
        let mut start = libc::timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        libc::gettimeofday(&mut start, std::ptr::null_mut());
        loop {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n > 0 {
                if contains_ethercat_frame(&buf[..n as usize]) {
                    return Ok(());
                }
            } else if n < 0 {
                let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                if e != libc::EAGAIN && e != libc::EWOULDBLOCK {
                    error!(
                        "BPF read on '{}': {}",
                        interface_name,
                        std::io::Error::last_os_error()
                    );
                    return Err(anyhow::anyhow!(
                        "BPF read: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }
            let mut now = libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            };
            libc::gettimeofday(&mut now, std::ptr::null_mut());
            let elapsed =
                (now.tv_sec - start.tv_sec) * 1_000_000 + (now.tv_usec - start.tv_usec) as i64;
            if elapsed >= 500_000 {
                warn!("No EtherCAT response on '{}' within 500ms", interface_name);
                return Err(anyhow::anyhow!("No EtherCAT response on {interface_name}"));
            }
            libc::nanosleep(
                &libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 1_000_000,
                },
                std::ptr::null_mut(),
            );
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
