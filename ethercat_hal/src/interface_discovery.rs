use libc::{freeifaddrs, getifaddrs, ifaddrs};
use std::ffi::CStr;
use std::ptr;
#[cfg(target_os = "linux")]
use std::{ffi::CString, mem, os::fd::RawFd};

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
    // getifaddrs populates a linked list of interface structures.
    unsafe {
        if getifaddrs(&mut ifaddr) == -1 {
            eprintln!("Error calling getifaddrs");
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
    if test_discovery(fd, &ETHERCAT_DISCOVERY_FRAME) {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Interface {:?} is not Ethercat",
            interface_name
        ))
    }
}

// ── macOS BPF (Berkeley Packet Filter) ────────────────────────────────
// Native raw-packet I/O via /dev/bpfN.  No ethercrab / AF_PACKET required.

#[cfg(target_os = "macos")]
use std::{ffi::CString, mem, os::fd::RawFd};

#[cfg(target_os = "macos")]
const BIOCSETIF: libc::c_ulong = 0x8020426C;
#[cfg(target_os = "macos")]
const BIOCIMMEDIATE: libc::c_ulong = 0x80044272;
#[cfg(target_os = "macos")]
const IFNAMSIZ: usize = 16;

/// Matches the real `struct ifreq` (144 bytes on 64-bit macOS).
#[cfg(target_os = "macos")]
#[repr(C)]
struct Ifreq {
    ifr_name: [u8; IFNAMSIZ],
    _pad: [u8; 128],
}

#[cfg(target_os = "macos")]
fn open_bpf(interface_name: &str) -> Result<RawFd, anyhow::Error> {
    unsafe {
        for i in 0..256 {
            let dev = CString::new(format!("/dev/bpf{i}")).unwrap();
            let fd = libc::open(dev.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK);
            if fd < 0 {
                continue;
            }
            // Immediate mode — read returns as soon as a packet arrives.
            libc::ioctl(fd, BIOCIMMEDIATE, &1u32);

            let mut ifr: Ifreq = mem::zeroed();
            let name = interface_name.as_bytes();
            let n = name.len().min(IFNAMSIZ - 1);
            ifr.ifr_name[..n].copy_from_slice(&name[..n]);

            if libc::ioctl(fd, BIOCSETIF, &ifr) == -1 {
                libc::close(fd);
                continue;
            }
            return Ok(fd);
        }
        Err(anyhow::anyhow!("No free BPF device"))
    }
}

#[cfg(target_os = "macos")]
pub fn test_interface(interface_name: &str) -> Result<(), anyhow::Error> {
    // EtherCAT broadcast discovery frame padded to Ethernet minimum (60 bytes).
    const FRAME: [u8; 60] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x02, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x88, 0xa4, 0x0d, 0x10, 0x08, 0x01, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let fd = open_bpf(interface_name)?;
    let result = unsafe {
        let mut buf = [0u8; 4096];

        // Drain stale packets (non-blocking)
        while libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) > 0 {}

        // Send discovery frame
        if libc::write(fd, FRAME.as_ptr() as *const libc::c_void, FRAME.len()) < 0 {
            return Err(anyhow::anyhow!(
                "BPF write: {}",
                std::io::Error::last_os_error()
            ));
        }

        // Wait for response (500ms timeout, non-blocking read + nanosleep)
        let mut start = libc::timeval { tv_sec: 0, tv_usec: 0 };
        libc::gettimeofday(&mut start, std::ptr::null_mut());
        let received = loop {
            let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
            if n > 0 {
                break n;
            }
            if n < 0 {
                let e = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
                if e != libc::EAGAIN && e != libc::EWOULDBLOCK {
                    return Err(anyhow::anyhow!(
                        "BPF read: {}",
                        std::io::Error::last_os_error()
                    ));
                }
            }
            let mut now = libc::timeval { tv_sec: 0, tv_usec: 0 };
            libc::gettimeofday(&mut now, std::ptr::null_mut());
            let elapsed =
                (now.tv_sec - start.tv_sec) * 1_000_000 + (now.tv_usec - start.tv_usec) as i64;
            if elapsed >= 500_000 {
                return Err(anyhow::anyhow!(
                    "No EtherCAT response on {interface_name}"
                ));
            }
            libc::nanosleep(
                &libc::timespec { tv_sec: 0, tv_nsec: 1_000_000 },
                std::ptr::null_mut(),
            );
        };

        // Parse BPF header: macOS ARM64 uses timeval32 (18-byte header),
        // bh_hdrlen is a u16 at offset 16.
        let hdrlen_raw = u16::from_le_bytes([buf[16], buf[17]]) as usize;
        let hdrlen = if hdrlen_raw < 14 || hdrlen_raw > 256 { 18 } else { hdrlen_raw };
        if (received as usize) <= hdrlen {
            return Err(anyhow::anyhow!("No packet data in BPF response"));
        }
        let pkt = &buf[hdrlen..];
        if pkt.len() >= 14 && pkt[12] == 0x88 && pkt[13] == 0xa4 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("{interface_name:?} is not Ethercat"))
        }
    };
    unsafe { libc::close(fd); }
    result
}

// ── Other platforms (neither Linux nor macOS) ─────────────────────────
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn test_interface(interface_name: &str) -> Result<(), anyhow::Error> {
    Err(anyhow::anyhow!(
        "EtherCAT interface discovery is not available on this platform (interface: {})",
        interface_name
    ))
}
