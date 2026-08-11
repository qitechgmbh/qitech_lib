use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

/// Factory default for register `0701h`, the port the XTREM *listens* on.
/// The host sends its requests here.
pub const DEFAULT_DEVICE_LOCAL_PORT: u16 = 5555;

/// Factory default for register `0700h`, the port the XTREM *sends* to.
/// The host binds this port to receive responses and stream data.
pub const DEFAULT_DEVICE_REMOTE_PORT: u16 = 4444;

/// Open the receive socket in non-blocking mode with broadcast enabled.
///
/// Binding `0.0.0.0` is deliberate: the modules answer to the port configured in register
/// `0700h` rather than to the source port of the request, so the socket has to be reachable
/// on every local address.
pub fn bind_socket(bind_addr: SocketAddrV4) -> io::Result<UdpSocket> {
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
