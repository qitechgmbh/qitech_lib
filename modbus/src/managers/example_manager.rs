use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use tokio::sync::{
    mpsc::{self, Sender,Receiver},
    oneshot,
};
use tokio_modbus::{ExceptionCode, Response};

use crate::{Device, Priority, Scheduler};

use crate::clients::example_client::{RequestMessage};

pub struct ExampleDeviceManager {
    tx: mpsc::Sender<RequestMessage>,

    devices: HashMap<u8, (Sender<Result<Response,ExceptionCode>>,Receiver<Result<Response,ExceptionCode>>, Rc<RefCell<dyn Device<ExampleScheduler>>>) >,
    scheduled_devices: VecDeque<u8>,
    pending_response: Option<u8>,
}

pub struct ExampleScheduler {
    id: u8,
    mgr: Rc<RefCell<ExampleDeviceManager>>,
}

impl Scheduler for ExampleScheduler {
    fn schedule(&self, priority: Priority) {
        _ = priority;

        self.mgr.borrow_mut().scheduled_devices.push_back(self.id);
    }
}

impl ExampleDeviceManager {
    pub fn new(tx: mpsc::Sender<RequestMessage>) -> Rc<RefCell<ExampleDeviceManager>> {
        let instance = Self {
            devices: HashMap::default(),
            scheduled_devices: VecDeque::default(),
            pending_response: None,
            tx,
        };

        Rc::new(RefCell::new(instance))
    }

    pub fn register_device<D>(
        mgr_rc: Rc<RefCell<Self>>, // <- pass Rc of self here
        slave_id: u8,
    ) -> Rc<RefCell<D>>
    where
        D: Device<ExampleScheduler> + 'static,
    {
        let scheduler = ExampleScheduler {
            id: slave_id,
            mgr: mgr_rc.clone(), // now you can safely store Rc
        };

        let device = Rc::new(RefCell::new(D::new(scheduler)));
        let (tx,rx) = tokio::sync::mpsc::channel(2);
        mgr_rc.borrow_mut().devices.insert(slave_id,( tx,rx,device.clone()) );
        device
    }

    pub fn update(&mut self) {
        self.try_receive();
        self.try_send();
    }

    pub fn try_receive(&mut self) {
        if let Some(id) = &mut self.pending_response {
            let device_tuple = self.devices.get_mut(&id).unwrap();
            let result = match device_tuple.1.try_recv() {
                Ok(v) => v,
                Err(_) => return,
            };

            let response = match result {
                Ok(v) => v,
                Err(e) => {
                    println!("Received exception code: {}", e);
                    return;
                }
            };
            let device_tuple = self.devices.get(&id).unwrap();
            let mut device = device_tuple.2.borrow_mut();
            if let Err(e) = device.handle_response(response) {
                println!("Error received while device processed response {:?}", e);
            }
        }
    }

    pub fn try_send(&mut self) {
        if let Some(id) = self.scheduled_devices.pop_front() {
            let device = self.devices.get(&id).unwrap();

            let (request, has_more) = device.2.borrow_mut().next_request().unwrap();


            let res = self.tx.try_send((id, request, device.0.clone()));
            if res.is_err() {
                return;
            }
            self.pending_response = Some(id);
            if has_more {
                self.scheduled_devices.push_front(id);
            }
        }
    }
}
