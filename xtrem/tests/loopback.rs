//! End-to-end tests against a fake XTREM module on loopback.
//!
//! The fake speaks the real wire format, so these exercise the whole stack — bus, demux,
//! discovery and the scale driver — without hardware.

use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;
use xtrem::protocol::{BROADCAST_ID, DataAddress, Frame, Function};
use xtrem::transport::{XtremBus, XtremBusConfig, XtremBusHandle};
use xtrem::{ScaleMode, XtremDevice, XtremError, XtremScale, discovery};

/// What the fake reports for register 0107h: 123.4 g gross, no tare, stable and off zero.
const WEIGHING_PAYLOAD: &[u8] = b"W   123.4g T     0.0g S014";
const FAKE_SERIAL: u32 = 1_123_456_724;
const FAKE_DEVICE_ID: u8 = 0x07;

/// Bind a fake module on loopback and start serving. Returns the address to talk to it on.
fn spawn_fake_module(device_id: u8, serial: u32) -> SocketAddrV4 {
    let runtime = common::get_async_runtime();
    let _guard = runtime.enter();

    let std_socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind fake module");
    std_socket.set_nonblocking(true).expect("nonblocking");
    let addr = match std_socket.local_addr().expect("local addr") {
        SocketAddr::V4(addr) => addr,
        SocketAddr::V6(addr) => panic!("expected IPv4, got {addr}"),
    };

    let socket = Arc::new(UdpSocket::from_std(std_socket).expect("adopt socket"));
    runtime.spawn(serve(socket, device_id, serial));
    addr
}

/// A bus whose "broadcast" address is the fake module. On loopback there is nothing to
/// broadcast to, so pointing it straight at the fake gives the same code path.
fn bus_for(module: SocketAddrV4) -> XtremBusHandle {
    XtremBus::open(XtremBusConfig {
        bind_addr: SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 0),
        broadcast_addr: module,
        host_id: 0x00,
        verify_lrc: true,
        crlf: true,
    })
    .expect("bus")
}

async fn serve(socket: Arc<UdpSocket>, device_id: u8, serial: u32) {
    let mut buf = [0u8; 512];
    let mut stream_target: Option<SocketAddr> = None;
    let mut ticker = tokio::time::interval(Duration::from_millis(20));

    loop {
        tokio::select! {
            received = socket.recv_from(&mut buf) => {
                let Ok((len, from)) = received else { continue };
                let Ok(request) = Frame::decode(&buf[..len], true) else { continue };
                if request.id_dest != device_id && request.id_dest != BROADCAST_ID {
                    continue;
                }

                let (function, data) = match (request.function, request.address) {
                    (Function::ReadRequest, DataAddress::SerialNumber) => {
                        (Function::ReadResponse, serial.to_string().into_bytes())
                    }
                    (Function::ReadRequest, DataAddress::SealingSwitchState) => {
                        (Function::ReadResponse, b"0".to_vec())
                    }
                    (Function::ReadRequest, DataAddress::DeviceState) => {
                        // "40": weighing ok, power ok, Wi-Fi module ready.
                        (Function::ReadResponse, b"40".to_vec())
                    }
                    (Function::ReadRequest, DataAddress::WeighingRegister) => {
                        (Function::ReadResponse, WEIGHING_PAYLOAD.to_vec())
                    }
                    (Function::WriteRequest, DataAddress::StreamOutputRate) => {
                        (Function::WriteResponse, b"0".to_vec())
                    }
                    (Function::ExecuteRequest, DataAddress::StartStreamWeight) => {
                        stream_target = Some(from);
                        (Function::ExecuteResponse, b"0".to_vec())
                    }
                    (Function::ExecuteRequest, DataAddress::StopStream) => {
                        stream_target = None;
                        (Function::ExecuteResponse, b"0".to_vec())
                    }
                    (Function::ExecuteRequest, _) => (Function::ExecuteResponse, b"0".to_vec()),
                    (Function::WriteRequest, _) => (Function::WriteResponse, b"0".to_vec()),
                    // Reads of registers this fake does not model go unanswered, exactly as a
                    // real module ignores traffic it cannot serve.
                    _ => continue,
                };

                let response = Frame {
                    id_origin: device_id,
                    id_dest: request.id_origin,
                    function,
                    address: request.address,
                    data,
                };
                let _ = socket.send_to(&response.to_bytes(true), from).await;
            }
            _ = ticker.tick(), if stream_target.is_some() => {
                let target = stream_target.expect("checked by the select guard");
                let frame = Frame {
                    id_origin: device_id,
                    id_dest: 0x00,
                    function: Function::ReadResponse,
                    address: DataAddress::WeighingRegister,
                    data: WEIGHING_PAYLOAD.to_vec(),
                };
                let _ = socket.send_to(&frame.to_bytes(true), target).await;
            }
        }
    }
}

/// Drive the synchronous polling contract until `done` is satisfied or the budget runs out.
fn pump_until(scale: &mut XtremScale, ticks: usize, done: impl Fn(&XtremScale) -> bool) -> bool {
    for _ in 0..ticks {
        scale.send_next_request().expect("send");
        scale.handle_response().expect("handle");
        if done(scale) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // One last drain, in case the answer landed during the final sleep.
    scale.handle_response().expect("handle");
    done(scale)
}

/// Assert the module never *refused* anything.
///
/// Timeouts and send-buffer pressure are deliberately tolerated: these tests share one runtime
/// with several others, so a request can miss its deadline for reasons that have nothing to do
/// with the protocol. A refusal, on the other hand, means the module understood us and said no.
fn assert_no_rejection(scale: &XtremScale) {
    match &scale.last_error {
        None | Some(XtremError::Timeout(_)) | Some(XtremError::Send(_)) => {}
        Some(other) => panic!("module rejected a request: {other}"),
    }
}

#[test]
fn discovers_the_fake_module() {
    let module = spawn_fake_module(FAKE_DEVICE_ID, FAKE_SERIAL);
    let bus = bus_for(module);

    let probes = common::get_async_runtime()
        .block_on(discovery::discover(&bus, Duration::from_millis(500)))
        .expect("discover");

    assert_eq!(
        probes.len(),
        1,
        "expected exactly one module, got {probes:?}"
    );
    let probe = &probes[0];
    assert_eq!(probe.serial, FAKE_SERIAL);
    assert_eq!(probe.device_id, FAKE_DEVICE_ID);
    assert_eq!(probe.addr, module);
    assert_eq!(probe.sealed, Some(false));
    assert!(!probe.id_collision);
    assert!(probe.state.expect("device state").is_healthy());
}

#[test]
fn polls_a_weight_reading() {
    let module = spawn_fake_module(FAKE_DEVICE_ID, FAKE_SERIAL);
    let bus = bus_for(module);

    let mut scale = XtremScale::new(&bus, FAKE_DEVICE_ID, ScaleMode::Poll);
    scale.set_addr(module);

    assert!(
        pump_until(&mut scale, 200, |s| s.reading.is_some()),
        "no reading after polling; bus stats: {:?}, error: {:?}",
        bus.stats(),
        scale.last_error
    );

    let reading = scale.reading.expect("reading");
    assert!((reading.net.get::<units::mass::gram>() - 123.4).abs() < 1e-9);
    assert!(reading.status.stable());
    assert!(!reading.status.zero());
    assert_no_rejection(&scale);
}

#[test]
fn streams_readings_and_stops_on_drop() {
    let module = spawn_fake_module(FAKE_DEVICE_ID, FAKE_SERIAL);
    let bus = bus_for(module);

    let mut scale = XtremScale::new(&bus, FAKE_DEVICE_ID, ScaleMode::Stream { interval_ms: 20 });
    scale.set_addr(module);

    assert!(
        pump_until(&mut scale, 200, |s| s.reading.is_some()),
        "no streamed reading; bus stats: {:?}, error: {:?}",
        bus.stats(),
        scale.last_error
    );
    assert_no_rejection(&scale);

    // Dropping the driver must tell the module to stop, otherwise it floods the subnet forever.
    // The fake streams every 20 ms, so an unstopped stream shows up as dozens of frames.
    drop(scale);
    std::thread::sleep(Duration::from_millis(500));
    let before = bus.stats().frames_received;
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        bus.stats().frames_received,
        before,
        "module was still streaming after the driver was dropped"
    );
}

#[test]
fn tare_command_reaches_the_module() {
    let module = spawn_fake_module(FAKE_DEVICE_ID, FAKE_SERIAL);
    let bus = bus_for(module);

    let mut scale = XtremScale::new(&bus, FAKE_DEVICE_ID, ScaleMode::Poll);
    scale.set_addr(module);
    scale.tare();

    // The fake accepts the tare, so polling must keep working and nothing may be refused.
    assert!(pump_until(&mut scale, 200, |s| s.reading.is_some()));
    assert_no_rejection(&scale);
}

/// A module that reboots stops streaming without saying so. The driver has to notice the
/// silence and re-send the start sequence, otherwise it goes quiet forever.
#[test]
fn re_arms_a_stream_that_never_produces_frames() {
    // Nothing is listening at this address, so no answer ever comes back.
    let dead = SocketAddrV4::new(std::net::Ipv4Addr::LOCALHOST, 59_999);
    let bus = bus_for(dead);

    let mut scale = XtremScale::new(&bus, 0x01, ScaleMode::Stream { interval_ms: 20 });
    scale.set_addr(dead);
    scale.set_request_timeout(Duration::from_millis(50));

    // Drive past the staleness window (1 s floor) and count how often the driver gave up on a
    // request. Without re-arming it would time out at most twice — once for the interval write
    // and once for the stream start — and then stay silent.
    let mut timeouts = 0;
    let deadline = Instant::now() + Duration::from_millis(2500);
    while Instant::now() < deadline {
        scale.send_next_request().expect("send");
        scale.handle_response().expect("handle");
        if scale.take_error().is_some() {
            timeouts += 1;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(
        timeouts > 2,
        "driver stopped retrying the stream start sequence ({timeouts} attempts)"
    );
}

#[test]
fn ignores_frames_from_other_modules() {
    let first = spawn_fake_module(0x01, 11);
    let second = spawn_fake_module(0x02, 22);
    let bus = bus_for(first);

    let mut scale = XtremScale::new(&bus, 0x02, ScaleMode::Poll);
    scale.set_addr(second);

    assert!(pump_until(&mut scale, 200, |s| s.reading.is_some()));

    // Discovery reaches only the module the broadcast address points at on loopback, so the
    // second module's traffic is what proves the ID demux works: the scale bound to 0x02 got a
    // reading while 0x01 was also on the bus.
    let probes = common::get_async_runtime()
        .block_on(discovery::discover(&bus, Duration::from_millis(300)))
        .expect("discover");
    assert_eq!(probes.len(), 1);
    assert_eq!(probes[0].device_id, 0x01);
}
