use bitvec::field::BitField;
use bitvec::prelude::{BitSlice, Lsb0};
use ethercat_hal_derive::{PdoObject, RxPdo, TxPdo};

use crate::io::multi_timestamp::MultiTimestampEvent;
use crate::pdo::{RxPdoObject, TxPdoObject};

const EXPECT_TEXT: &str = "All channels should be Some(_)";

#[derive(Debug, RxPdo)]
pub(super) struct EL1259RxPdo {
    #[pdo_object_index(0x1600)]
    pub(super) mto_channel1: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x1604)]
    pub(super) mto_channel2: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x1608)]
    pub(super) mto_channel3: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x160C)]
    pub(super) mto_channel4: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x1610)]
    pub(super) mto_channel5: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x1614)]
    pub(super) mto_channel6: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x1618)]
    pub(super) mto_channel7: Option<EL1259MtoRxChannel>,
    #[pdo_object_index(0x161C)]
    pub(super) mto_channel8: Option<EL1259MtoRxChannel>,

    #[pdo_object_index(0x1620)]
    pub(super) mti_channel1: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1621)]
    pub(super) mti_channel2: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1622)]
    pub(super) mti_channel3: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1623)]
    pub(super) mti_channel4: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1624)]
    pub(super) mti_channel5: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1625)]
    pub(super) mti_channel6: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1626)]
    pub(super) mti_channel7: Option<EL1259MtiRxChannel>,
    #[pdo_object_index(0x1627)]
    pub(super) mti_channel8: Option<EL1259MtiRxChannel>,
}

impl Default for EL1259RxPdo {

    fn default() -> Self {
        Self {
            mto_channel1: Some(EL1259MtoRxChannel::default()),
            mto_channel2: Some(EL1259MtoRxChannel::default()),
            mto_channel3: Some(EL1259MtoRxChannel::default()),
            mto_channel4: Some(EL1259MtoRxChannel::default()),
            mto_channel5: Some(EL1259MtoRxChannel::default()),
            mto_channel6: Some(EL1259MtoRxChannel::default()),
            mto_channel7: Some(EL1259MtoRxChannel::default()),
            mto_channel8: Some(EL1259MtoRxChannel::default()),

            mti_channel1: Some(EL1259MtiRxChannel::default()),
            mti_channel2: Some(EL1259MtiRxChannel::default()),
            mti_channel3: Some(EL1259MtiRxChannel::default()),
            mti_channel4: Some(EL1259MtiRxChannel::default()),
            mti_channel5: Some(EL1259MtiRxChannel::default()),
            mti_channel6: Some(EL1259MtiRxChannel::default()),
            mti_channel7: Some(EL1259MtiRxChannel::default()),
            mti_channel8: Some(EL1259MtiRxChannel::default()),
        }
    }
}

impl EL1259RxPdo {

    pub(super) fn get_mti(&self, channel: usize) -> &EL1259MtiRxChannel {
        match channel {
            0 => self.mti_channel1.as_ref().expect(EXPECT_TEXT),
            1 => self.mti_channel2.as_ref().expect(EXPECT_TEXT),
            2 => self.mti_channel3.as_ref().expect(EXPECT_TEXT),
            3 => self.mti_channel4.as_ref().expect(EXPECT_TEXT),
            4 => self.mti_channel5.as_ref().expect(EXPECT_TEXT),
            5 => self.mti_channel6.as_ref().expect(EXPECT_TEXT),
            6 => self.mti_channel7.as_ref().expect(EXPECT_TEXT),
            7 => self.mti_channel8.as_ref().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }

    pub(super) fn get_mti_mut(&mut self, channel: usize) -> &mut EL1259MtiRxChannel {
        match channel {
            0 => self.mti_channel1.as_mut().expect(EXPECT_TEXT),
            1 => self.mti_channel2.as_mut().expect(EXPECT_TEXT),
            2 => self.mti_channel3.as_mut().expect(EXPECT_TEXT),
            3 => self.mti_channel4.as_mut().expect(EXPECT_TEXT),
            4 => self.mti_channel5.as_mut().expect(EXPECT_TEXT),
            5 => self.mti_channel6.as_mut().expect(EXPECT_TEXT),
            6 => self.mti_channel7.as_mut().expect(EXPECT_TEXT),
            7 => self.mti_channel8.as_mut().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }

    pub(super) fn get_mto(&self, channel: usize) -> &EL1259MtoRxChannel {
        match channel {
            0 => self.mto_channel1.as_ref().expect(EXPECT_TEXT),
            1 => self.mto_channel2.as_ref().expect(EXPECT_TEXT),
            2 => self.mto_channel3.as_ref().expect(EXPECT_TEXT),
            3 => self.mto_channel4.as_ref().expect(EXPECT_TEXT),
            4 => self.mto_channel5.as_ref().expect(EXPECT_TEXT),
            5 => self.mto_channel6.as_ref().expect(EXPECT_TEXT),
            6 => self.mto_channel7.as_ref().expect(EXPECT_TEXT),
            7 => self.mto_channel8.as_ref().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }

    pub(super) fn get_mto_mut(&mut self, channel: usize) -> &mut EL1259MtoRxChannel {
        match channel {
            0 => self.mto_channel1.as_mut().expect(EXPECT_TEXT),
            1 => self.mto_channel2.as_mut().expect(EXPECT_TEXT),
            2 => self.mto_channel3.as_mut().expect(EXPECT_TEXT),
            3 => self.mto_channel4.as_mut().expect(EXPECT_TEXT),
            4 => self.mto_channel5.as_mut().expect(EXPECT_TEXT),
            5 => self.mto_channel6.as_mut().expect(EXPECT_TEXT),
            6 => self.mto_channel7.as_mut().expect(EXPECT_TEXT),
            7 => self.mto_channel8.as_mut().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }
}

/// This PDO object is used to send up to ten output
/// events to the multi-timestamp output channel.
/// Each event consists of a timestamp showing when the
/// event is applied and a corresponding output value.
#[derive(Debug, Default, PdoObject)]
#[pdo_object(bits = 384)]
pub(super) struct EL1259MtoRxChannel {
    /// If set, clear the event buffer.
    pub(super) output_buffer_reset: bool,
    /// While the `enable_manual_operation` flag is set,
    /// this channel will output this value regardless of any events in the buffer.
    pub(super) manual_output_state: bool,
    /// If set, all buffered events will be applied in sequence.
    /// This also means, that the latest event will allways be applied,
    /// even if it is in the past.
    pub(super) force_order: bool,
    /// If set, overwrite the current output with the value of `manual_output_state`
    /// regardless of any events in the buffer.
    pub(super) enable_manual_operation: bool,
    /// Incrementing this byte tells this channel that new events
    /// should be read and put into the buffer.
    /// Once all events have been read, this value of `EL1259MtoTxChannel::output_order_feedback`
    /// from this channel's MTO input will refect the new value.
    pub(super) output_order_count: u8,
    /// Number of valid entries in `output_events`.
    pub(super) number_of_output_events: u8,
    /// Output events to be send to this channel.
    /// A total of `number_of_output_events` will be added
    /// to this channel's internal buffer.
    /// Remaining entries will be ignored by this channel.
    pub(super) output_events: [MultiTimestampEvent; 10],
}

impl RxPdoObject for EL1259MtoRxChannel {

    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        // 0x7001:01..04 (bit 0..3)
        bits.set(0, self.output_buffer_reset);
        bits.set(1, self.manual_output_state);
        bits.set(2, self.force_order);
        bits.set(3, self.enable_manual_operation);

        // 0x7001:09 (bit 8..15)
        bits[8..16].store_le(self.output_order_count);

        // 0x7001:17 (bit 16..23)
        bits[16..24].store_le(self.number_of_output_events);

        for i in 0..self.number_of_output_events as usize {
            // 0x7001:21..2A (bit 32..41)
            bits.set(32, self.output_events[i].value);

            // 0x7001:41..4A (bit 64..383)
            let offset = i * 32;
            bits[64+offset .. 96+offset].store_le(self.output_events[i].dc_timestamp_ns);
        }
    }
}

impl EL1259MtoRxChannel {

    pub(super) fn set_events(&mut self, events: &[MultiTimestampEvent]) {
        let len = events.len();
        assert!(len <= 10);
        self.number_of_output_events = len as u8;
        self.output_events[..len].clone_from_slice(events);
        println!("self.number_of_output_events={}, self.output_events={:?}", self.number_of_output_events, self.output_events);
    }
}

/// This PDO object controls the multi-timestamp input channel.
/// Importantly, it is used to tell this channel when to send input events.
#[derive(Debug, Default, PdoObject)]
#[pdo_object(bits = 32)]
pub(super) struct EL1259MtiRxChannel {
    /// If set, clear the event buffer.
    pub(super) input_buffer_reset: bool,
    /// Incrementing this byte informs this channel that new input events can be sent via the `EL1259MtiTxChannel`.
    /// Once events have been sent, the value of this channel's `EL1259MtiTxChannel::input_order_feedback`
    /// will reflect the new value, and events can be read from that PDS.
    pub(super) input_order_counter: u8,
}

impl RxPdoObject for EL1259MtiRxChannel {

    fn write(&self, bits: &mut BitSlice<u8, Lsb0>) {
        // 0x7080:01 (bit 0)
        bits.set(0, self.input_buffer_reset);

        // 0x7080:11 (bit 16..23)
        bits[16..24].store_le(self.input_order_counter);
    }
}

#[derive(Debug, TxPdo)]
pub(super) struct EL1259TxPdo {
    #[pdo_object_index(0x1A00)]
    pub(super) mto_channel1: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A01)]
    pub(super) mto_channel2: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A02)]
    pub(super) mto_channel3: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A03)]
    pub(super) mto_channel4: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A04)]
    pub(super) mto_channel5: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A05)]
    pub(super) mto_channel6: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A06)]
    pub(super) mto_channel7: Option<EL1259MtoTxChannel>,
    #[pdo_object_index(0x1A07)]
    pub(super) mto_channel8: Option<EL1259MtoTxChannel>,

    #[pdo_object_index(0x1A08)]
    pub(super) mti_channel1: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A0C)]
    pub(super) mti_channel2: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A10)]
    pub(super) mti_channel3: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A14)]
    pub(super) mti_channel4: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A18)]
    pub(super) mti_channel5: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A1C)]
    pub(super) mti_channel6: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A20)]
    pub(super) mti_channel7: Option<EL1259MtiTxChannel>,
    #[pdo_object_index(0x1A24)]
    pub(super) mti_channel8: Option<EL1259MtiTxChannel>,
}

impl Default for EL1259TxPdo {

    fn default() -> Self {
        Self {
            mto_channel1: Some(EL1259MtoTxChannel::default()),
            mto_channel2: Some(EL1259MtoTxChannel::default()),
            mto_channel3: Some(EL1259MtoTxChannel::default()),
            mto_channel4: Some(EL1259MtoTxChannel::default()),
            mto_channel5: Some(EL1259MtoTxChannel::default()),
            mto_channel6: Some(EL1259MtoTxChannel::default()),
            mto_channel7: Some(EL1259MtoTxChannel::default()),
            mto_channel8: Some(EL1259MtoTxChannel::default()),

            mti_channel1: Some(EL1259MtiTxChannel::default()),
            mti_channel2: Some(EL1259MtiTxChannel::default()),
            mti_channel3: Some(EL1259MtiTxChannel::default()),
            mti_channel4: Some(EL1259MtiTxChannel::default()),
            mti_channel5: Some(EL1259MtiTxChannel::default()),
            mti_channel6: Some(EL1259MtiTxChannel::default()),
            mti_channel7: Some(EL1259MtiTxChannel::default()),
            mti_channel8: Some(EL1259MtiTxChannel::default()),
        }
    }
}

impl EL1259TxPdo {

    pub(super) fn get_mti(&self, channel: usize) -> &EL1259MtiTxChannel {
        match channel {
            0 => self.mti_channel1.as_ref().expect(EXPECT_TEXT),
            1 => self.mti_channel2.as_ref().expect(EXPECT_TEXT),
            2 => self.mti_channel3.as_ref().expect(EXPECT_TEXT),
            3 => self.mti_channel4.as_ref().expect(EXPECT_TEXT),
            4 => self.mti_channel5.as_ref().expect(EXPECT_TEXT),
            5 => self.mti_channel6.as_ref().expect(EXPECT_TEXT),
            6 => self.mti_channel7.as_ref().expect(EXPECT_TEXT),
            7 => self.mti_channel8.as_ref().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }

    pub(super) fn get_mti_mut(&mut self, channel: usize) -> &mut EL1259MtiTxChannel {
        match channel {
            0 => self.mti_channel1.as_mut().expect(EXPECT_TEXT),
            1 => self.mti_channel2.as_mut().expect(EXPECT_TEXT),
            2 => self.mti_channel3.as_mut().expect(EXPECT_TEXT),
            3 => self.mti_channel4.as_mut().expect(EXPECT_TEXT),
            4 => self.mti_channel5.as_mut().expect(EXPECT_TEXT),
            5 => self.mti_channel6.as_mut().expect(EXPECT_TEXT),
            6 => self.mti_channel7.as_mut().expect(EXPECT_TEXT),
            7 => self.mti_channel8.as_mut().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }

    pub(super) fn get_mto(&self, channel: usize) -> &EL1259MtoTxChannel {
        match channel {
            0 => self.mto_channel1.as_ref().expect(EXPECT_TEXT),
            1 => self.mto_channel2.as_ref().expect(EXPECT_TEXT),
            2 => self.mto_channel3.as_ref().expect(EXPECT_TEXT),
            3 => self.mto_channel4.as_ref().expect(EXPECT_TEXT),
            4 => self.mto_channel5.as_ref().expect(EXPECT_TEXT),
            5 => self.mto_channel6.as_ref().expect(EXPECT_TEXT),
            6 => self.mto_channel7.as_ref().expect(EXPECT_TEXT),
            7 => self.mto_channel8.as_ref().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }

    pub(super) fn get_mto_mut(&mut self, channel: usize) -> &mut EL1259MtoTxChannel {
        match channel {
            0 => self.mto_channel1.as_mut().expect(EXPECT_TEXT),
            1 => self.mto_channel2.as_mut().expect(EXPECT_TEXT),
            2 => self.mto_channel3.as_mut().expect(EXPECT_TEXT),
            3 => self.mto_channel4.as_mut().expect(EXPECT_TEXT),
            4 => self.mto_channel5.as_mut().expect(EXPECT_TEXT),
            5 => self.mto_channel6.as_mut().expect(EXPECT_TEXT),
            6 => self.mto_channel7.as_mut().expect(EXPECT_TEXT),
            7 => self.mto_channel8.as_mut().expect(EXPECT_TEXT),
            _ => panic!("Channel index out of range {}", channel),
        }
    }
}

/// This PDO object is used read the status of the multi-timestamp output channel.
/// Importantly, through this PDO, this channel reports back when it is again ready
/// to receive new output events.
#[derive(Debug, Default, PdoObject)]
#[pdo_object(bits = 32)]
pub(super) struct EL1259MtoTxChannel {
    /// If set, this channel detected a short circuit.
    pub(super) output_short_circuit: bool,
    /// If set, this channel's event buffer has overflown.
    pub(super) output_buffer_overflow: bool,
    /// This channel's value in the current cycle.
    pub(super) output_state: bool,
    /// 2-Bit counter counting the number of process cycles.
    /// Can be used to detect that this channel did not correctly
    /// process data in a cycle.
    pub(super) input_cycle_counter: u8,
    /// This number reflects the `EL1259MtoRxChannel::output_order_count` for this channel.
    /// If those two numbers are not sync, new output events can be send to this channel.
    /// Otherwise, this channel need additional cycles to store received input events
    /// into the internal buffer.
    pub(super) output_order_feedback: u8,
    /// Number of output events currently stored in this channel's event buffer.
    /// A maximum of 32 events can be stored at a time.
    pub(super) events_in_output_buffer: u8,
}

impl TxPdoObject for EL1259MtoTxChannel {

    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        // 0x6000:01..03 (bit 0..2)
        self.output_short_circuit   = bits[0];
        self.output_buffer_overflow = bits[1];
        self.output_state           = bits[2];

        // 0x6000:0F (bit 14..15)
        self.input_cycle_counter     = bits[14..16].load_le();

        // 0x6000:11 (bit 16..23)
        self.output_order_feedback   = bits[16..24].load_le();

        // 0x6000:12 (bit 24..31)
        self.events_in_output_buffer = bits[24..32].load_le();
    }
}

/// This PDO object is used ...
#[derive(Debug, Default, PdoObject)]
#[pdo_object(bits = 384)]
pub(super) struct EL1259MtiTxChannel {
    /// Number of valid entries in `input_events`.
    pub(super) number_of_input_events: u8,
    /// This channel's value in the current cycle.
    pub(super) input_state: bool,
    /// If set, the internal event buffer has overflown.
    pub(super) input_buffer_overflow: bool,
    /// 2-Bit counter counting the number of process cycles.
    /// Can be used to detect that this channel did not correctly
    /// process data in a cycle.
    pub(super) input_cycle_counter: u8,
    /// Number of output events currently stored in this channel's event buffer.
    /// A maximum of 32 events can be stored at a time.
    pub(super) events_in_input_buffer: u8,
    /// Reflects the value of `EL1259MtiRxChannel::input_order_counter`.
    /// If these two numbers are in sync, input events can be read from this PDO.
    /// Otherwise, this channel requires additional cycles to send input events.
    /// The field `number_of_input_events` indicates the number of valid event entries.
    /// After events have beed read, the value `EL1259MtiRxChannel::input_order_counter`
    /// should be incremented, so new events will be send.
    pub(super) input_order_feedback: u8,
    /// Input events received from by this channel.
    /// A total of `number_of_input_events` entries are valid.
    /// Remaining entries have undefied values.
    pub(super) input_events: [MultiTimestampEvent; 10],
}

impl TxPdoObject for EL1259MtiTxChannel {

    fn read(&mut self, bits: &BitSlice<u8, Lsb0>) {
        // 0x6081:01 (bit 0..7)
        self.number_of_input_events = bits[0..8].load_le();

        // 0x6081:09..0A (bit (8..9)
        self.input_state           = bits[8];
        self.input_buffer_overflow = bits[9];

        // 0x6081:0F (bit 14..15)
        self.input_cycle_counter    = bits[14..16].load_le();

        // 0x6081:11 (bit 16..23)
        self.events_in_input_buffer = bits[16..23].load_le();

        // 0x6081:12 (bit 24..31)
        self.input_order_feedback   = bits[24..31].load_le();

        for i in 0..self.number_of_input_events as usize {
            // 0x6081:21..2A (bit 32..41)
            self.input_events[i].value = bits[32 + i];
            // 0x6081:41..4A (bit 64..383)
            let offset = i * 32;
            self.input_events[i].dc_timestamp_ns = bits[64+offset ..96 + offset].load_le();
        }
    }
}

impl EL1259MtiTxChannel {

    pub(super) fn get_events(&self) -> &[MultiTimestampEvent] {
        &self.input_events[0..self.number_of_input_events as usize]
    }
}
