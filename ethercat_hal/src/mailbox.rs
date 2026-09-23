use crate::{
    ETHERCAT_TX_RX_SIZE,
    types::{Consumer, Producer},
};
use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};
unsafe impl Sync for Mailbox {}
unsafe impl Send for Mailbox {}

pub struct Mailbox {
    pub data: UnsafeCell<[u8; ETHERCAT_TX_RX_SIZE]>,
    pub full: AtomicBool,
}

impl Consumer for std::sync::Arc<Mailbox> {
    fn read(&mut self) -> Option<&[u8]> {
        // Cast the shared Arc pointer to a mutable reference to Mailbox
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe { (&mut *ptr).read() }
    }

    fn finish_read(&mut self) {
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe {
            (&mut *ptr).finish_read();
        }
    }
}

impl Producer for std::sync::Arc<Mailbox> {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe { (&mut *ptr).input_buffer_mut() }
    }

    fn publish(&mut self) {
        let ptr = std::sync::Arc::as_ptr(self) as *mut Mailbox;
        unsafe {
            (&mut *ptr).publish();
        }
    }
}

impl Consumer for Mailbox {
    fn read(&mut self) -> Option<&[u8]> {
        // Consumer only reads if `full` is true
        if !self.full.load(Ordering::Acquire) {
            return None;
        }
        unsafe { Some(&*self.data.get()) }
    }

    fn finish_read(&mut self) {
        // We are completely done reading.
        // We store `false` to release the buffer back to the producer.
        self.full.store(false, Ordering::Release);
    }
}

impl Producer for Mailbox {
    fn input_buffer_mut(&mut self) -> Option<&mut [u8; ETHERCAT_TX_RX_SIZE]> {
        // Producer only writes if `full` is false
        if self.full.load(Ordering::Acquire) {
            return None;
        }
        unsafe { Some(&mut *self.data.get()) }
    }

    fn publish(&mut self) {
        // We are completely done writing.
        // We store `true` to release the buffer to the consumer.
        self.full.store(true, Ordering::Release);
    }
}
