use super::udp;
use crate::protocol::{DataAddress, Frame, ProtocolError};
use common::get_async_runtime;
use std::collections::HashMap;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

/// Datagrams larger than this are impossible: `D_L` is 8 bits, so the longest legal frame is
/// 255 data bytes plus 17 bytes of framing.
const RECV_BUF_LEN: usize = 512;

/// How many frames may queue up for a device before the oldest are dropped.
const DEVICE_QUEUE_LEN: usize = 8;

/// Backlog of the "every frame" channel that discovery listens on.
const EVENT_QUEUE_LEN: usize = 256;

/// Where a frame should be sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    /// The subnet broadcast address from [`XtremBusConfig::broadcast_addr`]. Every module on
    /// the network receives it; the `ID_D` field decides which one answers.
    Broadcast,
    /// A specific module, at the address discovery learned for it. Preferred once known.
    Unicast(SocketAddrV4),
}

/// A frame that arrived on the bus.
#[derive(Debug, Clone)]
pub struct Inbound {
    pub frame: Frame,
    pub from: SocketAddrV4,
    pub at: Instant,
}

#[derive(Debug, Clone)]
pub struct XtremBusConfig {
    /// Local address to bind, normally `0.0.0.0:<register 0700h>`.
    pub bind_addr: SocketAddrV4,
    /// Subnet broadcast address plus the module listen port, `<x.x.x.255>:<register 0701h>`.
    pub broadcast_addr: SocketAddrV4,
    /// `ID_O` this host puts in the frames it sends.
    pub host_id: u8,
    /// Whether to reject frames whose LRC does not match. Turn off only when the modules have
    /// LRC checking disabled (register `0011h`).
    pub verify_lrc: bool,
    /// Append CR+LF after ETX. Required by the Wi-Fi / Ethernet module (spec §17).
    pub crlf: bool,
}

impl Default for XtremBusConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddrV4::new(
                std::net::Ipv4Addr::UNSPECIFIED,
                udp::DEFAULT_DEVICE_REMOTE_PORT,
            ),
            broadcast_addr: SocketAddrV4::new(
                std::net::Ipv4Addr::BROADCAST,
                udp::DEFAULT_DEVICE_LOCAL_PORT,
            ),
            host_id: 0x00,
            verify_lrc: true,
            crlf: true,
        }
    }
}

/// Counters describing what the receive task has seen. Useful for diagnosing a quiet bus.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BusStats {
    pub frames_received: u64,
    pub decode_errors: u64,
    /// Frames whose `ID_O` had no registered device.
    pub unrouted: u64,
    /// Frames discarded because a device's queue was full.
    pub dropped: u64,
    pub io_errors: u64,
}

#[derive(Default)]
struct Counters {
    frames_received: AtomicU64,
    decode_errors: AtomicU64,
    unrouted: AtomicU64,
    dropped: AtomicU64,
    io_errors: AtomicU64,
}

impl Counters {
    fn snapshot(&self) -> BusStats {
        BusStats {
            frames_received: self.frames_received.load(Ordering::Relaxed),
            decode_errors: self.decode_errors.load(Ordering::Relaxed),
            unrouted: self.unrouted.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            io_errors: self.io_errors.load(Ordering::Relaxed),
        }
    }
}

struct BusInner {
    socket: Arc<UdpSocket>,
    config: XtremBusConfig,
    routes: Mutex<HashMap<u8, mpsc::Sender<Inbound>>>,
    events: broadcast::Sender<Inbound>,
    counters: Counters,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for BusInner {
    fn drop(&mut self) {
        if let Some(task) = self.task.lock().unwrap_or_else(|e| e.into_inner()).take() {
            task.abort();
        }
    }
}

/// Owns the UDP socket and the task that demultiplexes incoming frames.
///
/// One bus serves every module on the subnet. A per-device socket would not work: the modules
/// reply to the port configured in register `0700h`, not to the source port of the request, so
/// all traffic converges on a single local port and has to be sorted out by `ID_O`.
pub struct XtremBus;

impl XtremBus {
    /// Bind the socket and spawn the receive task on the shared runtime.
    ///
    /// The bus itself is not handed back — callers work through [`XtremBusHandle`], and the
    /// receive task lives exactly as long as the last handle.
    pub fn open(config: XtremBusConfig) -> Result<XtremBusHandle, anyhow::Error> {
        let std_socket = udp::bind_socket(config.bind_addr)?;

        let runtime = get_async_runtime();
        let _guard = runtime.enter();
        let socket = Arc::new(UdpSocket::from_std(std_socket)?);
        let (events, _) = broadcast::channel(EVENT_QUEUE_LEN);

        let inner = Arc::new(BusInner {
            socket: Arc::clone(&socket),
            config,
            routes: Mutex::new(HashMap::new()),
            events: events.clone(),
            counters: Counters::default(),
            task: Mutex::new(None),
        });

        let task = runtime.spawn(receive_loop(Arc::clone(&inner)));
        *inner.task.lock().unwrap_or_else(|e| e.into_inner()) = Some(task);

        Ok(XtremBusHandle { inner })
    }
}

/// A cheap, clonable handle to the bus. The receive task stops when the last handle drops.
#[derive(Clone)]
pub struct XtremBusHandle {
    inner: Arc<BusInner>,
}

impl XtremBusHandle {
    pub fn config(&self) -> &XtremBusConfig {
        &self.inner.config
    }

    pub fn host_id(&self) -> u8 {
        self.inner.config.host_id
    }

    /// The address the socket actually bound to, which resolves an ephemeral port 0.
    pub fn local_addr(&self) -> Result<SocketAddrV4, anyhow::Error> {
        match self.inner.socket.local_addr()? {
            SocketAddr::V4(addr) => Ok(addr),
            SocketAddr::V6(addr) => Err(anyhow::anyhow!("bus bound to an IPv6 address: {addr}")),
        }
    }

    pub fn stats(&self) -> BusStats {
        self.inner.counters.snapshot()
    }

    /// Route frames whose `ID_O` is `device_id` to the returned receiver.
    ///
    /// Registering the same id twice replaces the earlier route, so the previous owner stops
    /// receiving. The queue is bounded — a consumer that stalls loses the oldest frames rather
    /// than stalling the whole bus, which is the right trade for a device that is polled
    /// continuously and only ever cares about the newest reading.
    pub fn subscribe(&self, device_id: u8) -> mpsc::Receiver<Inbound> {
        let (tx, rx) = mpsc::channel(DEVICE_QUEUE_LEN);
        self.routes().insert(device_id, tx);
        rx
    }

    pub fn unsubscribe(&self, device_id: u8) {
        self.routes().remove(&device_id);
    }

    /// Every decoded frame, regardless of `ID_O`. Discovery uses this because it does not know
    /// the ids it is about to hear from.
    pub fn events(&self) -> broadcast::Receiver<Inbound> {
        self.inner.events.subscribe()
    }

    /// Build a read request addressed from this host.
    pub fn read_frame(&self, id_dest: u8, address: DataAddress) -> Frame {
        Frame::read(self.host_id(), id_dest, address)
    }

    /// Build a write request addressed from this host.
    pub fn write_frame(&self, id_dest: u8, address: DataAddress, data: Vec<u8>) -> Frame {
        Frame::write(self.host_id(), id_dest, address, data)
    }

    /// Build an execute request addressed from this host.
    pub fn execute_frame(&self, id_dest: u8, address: DataAddress) -> Frame {
        Frame::execute(self.host_id(), id_dest, address)
    }

    /// Send without blocking. This is what a synchronous control loop should call.
    ///
    /// Returns `WouldBlock` if the socket's send buffer is full; the caller should treat that
    /// as "try again next tick" rather than as a failure.
    pub fn try_send(&self, frame: &Frame, destination: Destination) -> Result<(), anyhow::Error> {
        let bytes = frame.to_bytes(self.inner.config.crlf);
        let addr = self.resolve(destination);
        self.inner.socket.try_send_to(&bytes, addr.into())?;
        Ok(())
    }

    /// Send, waiting for socket writability. For async callers such as discovery.
    pub async fn send(&self, frame: &Frame, destination: Destination) -> Result<(), anyhow::Error> {
        let bytes = frame.to_bytes(self.inner.config.crlf);
        let addr = self.resolve(destination);
        self.inner.socket.send_to(&bytes, addr).await?;
        Ok(())
    }

    fn resolve(&self, destination: Destination) -> SocketAddrV4 {
        match destination {
            Destination::Broadcast => self.inner.config.broadcast_addr,
            Destination::Unicast(addr) => addr,
        }
    }

    fn routes(&self) -> std::sync::MutexGuard<'_, HashMap<u8, mpsc::Sender<Inbound>>> {
        self.inner.routes.lock().unwrap_or_else(|e| e.into_inner())
    }
}

async fn receive_loop(inner: Arc<BusInner>) {
    let mut buf = [0u8; RECV_BUF_LEN];

    loop {
        let (len, from) = match inner.socket.recv_from(&mut buf).await {
            Ok((len, SocketAddr::V4(from))) => (len, from),
            // The modules are IPv4-only; anything else is not ours.
            Ok((_, SocketAddr::V6(_))) => continue,
            Err(_) => {
                inner.counters.io_errors.fetch_add(1, Ordering::Relaxed);
                // A transient socket error must not kill the bus. Yield so a persistent one
                // cannot spin the runtime.
                tokio::task::yield_now().await;
                continue;
            }
        };

        let frame = match Frame::decode(&buf[..len], inner.config.verify_lrc) {
            Ok(frame) => frame,
            Err(ProtocolError::MissingStx) => {
                // Not an XTREM datagram at all; ignore silently, as the spec asks (§4).
                continue;
            }
            Err(_) => {
                inner.counters.decode_errors.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        inner
            .counters
            .frames_received
            .fetch_add(1, Ordering::Relaxed);

        let inbound = Inbound {
            frame,
            from,
            at: Instant::now(),
        };

        // Discovery and any observer see everything; failure just means nobody is listening.
        let _ = inner.events.send(inbound.clone());

        let route = inner
            .routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&inbound.frame.id_origin)
            .cloned();

        match route {
            Some(tx) => {
                if tx.try_send(inbound).is_err() {
                    inner.counters.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
            None => {
                inner.counters.unrouted.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
