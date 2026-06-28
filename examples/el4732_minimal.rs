use bitvec::slice::BitSlice;
use ethercat_hal::{
    EtherCATState,
    devices::{
        EthercatDevice, EthercatDeviceProcessing,
        el4732::{EL4732, EL4732_PRODUCT_ID, EL4732Port},
    },
    init_ethercat,
};
use std::{env, f32::consts::TAU, time::Duration};

/// Must match the DC cycle time configured by the master.
const BUS_CYCLE: Duration = Duration::from_micros(1000); // 1 ms

/// Oversample factor. One of: 1, 2, 3, 4, 5, 8, 10, 16, 20, 25, 32, 40, 50, 100.
const OVERSAMPLE: usize = 10; // 10 kHz update at a 1 ms cycle

const SINE_FREQ_HZ: f32 = 50.0;
const AMPLITUDE: f32 = 0.8; // fraction of full scale; 1.0 == +/-10 V

struct SineGen {
    phase: f32,
    phase_step: f32,
}

impl SineGen {
    fn new(freq_hz: f32, bus_cycle: Duration, oversample: usize) -> Self {
        let slot_dt = bus_cycle.as_secs_f32() / oversample as f32;
        Self {
            phase: 0.0,
            phase_step: TAU * freq_hz * slot_dt,
        }
    }

    /// Fill one bus cycle's block; `out.len()` must equal the oversample factor
    /// (the driver asserts this).
    fn fill(&mut self, out: &mut [f32], amplitude: f32) {
        for slot in out.iter_mut() {
            *slot = amplitude * self.phase.sin();
            self.phase += self.phase_step;
        }
        self.phase = self.phase.rem_euclid(TAU);
    }
}

fn main() {
    let interface = env::args().nth(1).expect("No interface name given");

    let mut el4732 = EL4732::new_with_oversample(OVERSAMPLE);

    let mut sine = SineGen::new(SINE_FREQ_HZ, BUS_CYCLE, OVERSAMPLE);
    let mut samples = vec![0.0f32; OVERSAMPLE];

    let mut eth_control = init_ethercat(&interface, None);

    eth_control
        .channel
        .request_state_change(EtherCATState::PreOp)
        .expect("Channel was not ready");
    loop {
        if matches!(eth_control.controller.state, EtherCATState::PreOp) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    for subdevice in eth_control.controller.get_subdevices() {
        if subdevice.product_id == EL4732_PRODUCT_ID {
            eth_control
                .channel
                .enable_dc_sync0(subdevice.device_address)
                .expect("Failed to enable DC sync");
        }
    }

    eth_control
        .channel
        .request_state_change(EtherCATState::Op)
        .expect("Channel was not ready");
    'wait_op: loop {
        std::thread::sleep(Duration::from_millis(10));
        for subdevice in eth_control.controller.get_subdevices() {
            if !subdevice.initialized {
                continue 'wait_op;
            }
        }
        break;
    }

    println!(
        "DC system start time {} ns",
        eth_control.controller.get_dc_system_time_ns()
    );

    loop {
        if let Some(output) = eth_control.app_handle.write_outputs() {
            sine.fill(&mut samples, AMPLITUDE);
            el4732.set_output_samples(EL4732Port::AO1 as usize, &samples);

            for subdevice in eth_control.controller.get_subdevices() {
                if subdevice.product_id == EL4732_PRODUCT_ID {
                    el4732
                        .output_pre_process()
                        .expect("Failed to prepare output");
                    let out = &mut output[subdevice.start_rx..subdevice.end_rx];
                    el4732
                        .output(BitSlice::from_slice_mut(out))
                        .expect("Failed to write output");
                }
            }
        }

        eth_control.app_handle.send_outputs();
        std::hint::spin_loop();
    }
}
