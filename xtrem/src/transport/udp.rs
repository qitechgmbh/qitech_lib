use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

/// Factory default for register `0701h`, the port the XTREM *listens* on.
/// The host sends its requests here.
///
/// Verified against real hardware 2026-08-11: earlier values here had this swapped with
/// [`DEFAULT_DEVICE_REMOTE_PORT`], which produced total silence (nothing on the wire was
/// wrong, the two sides were just listening on each other's assumed ports).
pub const DEFAULT_DEVICE_LOCAL_PORT: u16 = 4444;

/// Factory default for register `0700h`, the port the XTREM *sends* to.
/// The host binds this port to receive responses and stream data.
pub const DEFAULT_DEVICE_REMOTE_PORT: u16 = 5555;

/// Open the receive socket in non-blocking mode with broadcast enabled.
///
/// `bind_addr` must be `0.0.0.0` (a specific port, unspecified IP) on a real deployment — this
/// is a hard requirement, not a style preference. Verified against real hardware: the module
/// never unicasts its reply to the requester's source IP, it always sends to the subnet
/// broadcast address (`255.255.255.255`), even in response to a unicast request. A socket bound
/// to one specific LAN address silently never receives those — the kernel drops them before the
/// application ever sees them, with no error on either side. Binding `0.0.0.0` is the only way
/// to actually receive replies.
///
/// Loopback (`127.0.0.0/8`) is exempt: it's the standard pattern for a hermetic test fixture
/// that replies with a real unicast (see `tests/loopback.rs`), and loopback delivery has none of
/// the broadcast-drop behavior this check exists to catch.
pub fn bind_socket(bind_addr: SocketAddrV4) -> io::Result<UdpSocket> {
    if !bind_addr.ip().is_unspecified() && !bind_addr.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "xtrem bind address must be 0.0.0.0:<port>, got {bind_addr}: the module \
                 replies to the broadcast address, never to the requester's unicast IP, so a \
                 socket bound to a specific LAN interface address never receives anything back"
            ),
        ));
    }
    let socket = UdpSocket::bind(bind_addr)?;
    socket.set_broadcast(true)?;
    socket.set_nonblocking(true)?;
    Ok(socket)
}

/// The directed broadcast address of the subnet `ip` sits in.
///
/// `prefix_len` is the CIDR prefix, so a module on `192.168.4.17/24` yields `192.168.4.255`.
/// Prefixes above 32 are clamped, which turns the result into `ip` itself.
pub fn broadcast_addr_for(ip: Ipv4Addr, prefix_len: u8, port: u16) -> SocketAddrV4 {
    // A /32 shifts the mask entirely away, which `>>` would panic on.
    let host_mask = u32::MAX.checked_shr(u32::from(prefix_len)).unwrap_or(0);
    let broadcast = u32::from(ip) | host_mask;
    SocketAddrV4::new(Ipv4Addr::from(broadcast), port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_subnet_broadcast_addresses() {
        assert_eq!(
            broadcast_addr_for(Ipv4Addr::new(192, 168, 4, 17), 24, 5555),
            SocketAddrV4::new(Ipv4Addr::new(192, 168, 4, 255), 5555)
        );
        assert_eq!(
            broadcast_addr_for(Ipv4Addr::new(10, 0, 1, 5), 16, 4445),
            SocketAddrV4::new(Ipv4Addr::new(10, 0, 255, 255), 4445)
        );
        // A /32 has no host bits, so the "broadcast" is the address itself.
        assert_eq!(
            broadcast_addr_for(Ipv4Addr::new(10, 0, 1, 5), 32, 1),
            SocketAddrV4::new(Ipv4Addr::new(10, 0, 1, 5), 1)
        );
    }
}
