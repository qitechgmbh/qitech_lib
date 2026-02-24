use std::{ cell::{RefCell}, collections::{HashMap, VecDeque}, rc::Rc };

use tokio::sync::{ mpsc::{ self }, oneshot };

use crate::{
    Device, 
    Priority, 
    Scheduler, 
};

use crate::clients::example_client::{RequestMessage, ResponseMessage};

pub struct ExampleDeviceManager
{
    tx: mpsc::Sender<RequestMessage>,

    devices: HashMap<u8, Rc<RefCell<dyn Device<ExampleScheduler>>>>,

    scheduled_devices: VecDeque<u8>,
    pending_response:   Option<(u8, oneshot::Receiver<ResponseMessage>)>,
}

pub struct ExampleScheduler
{
    id:  u8,
    mgr: Rc<RefCell<ExampleDeviceManager>>
}

impl Scheduler for ExampleScheduler
{
    fn schedule(&self, priority: Priority) 
    {
        _ = priority;

        self.mgr.borrow_mut().scheduled_devices.push_back(self.id);
    }
}

impl ExampleDeviceManager 
{
    pub fn new(tx: mpsc::Sender<RequestMessage>) -> Rc<RefCell<ExampleDeviceManager>>
    {
        let instance = Self { 
            devices:            HashMap::default(), 
            scheduled_devices: VecDeque::default(), 
            pending_response:   None, 
            tx 
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

        mgr_rc.borrow_mut().devices.insert(slave_id, device.clone());

        device
    }

    pub fn update(&mut self)
    {
        self.try_receive();
        self.try_send();
    }

    fn try_receive(&mut self)
    {
        if let Some((id, rx)) = &mut self.pending_response
        {
            let result = match rx.try_recv()
            {
                Ok(v)  => v,
                Err(_) => return,
            };

            let response = match result 
            {
                Ok(v)  => v,
                Err(e) => 
                {
                    println!("Received exception code: {}", e);
                    return;
                },
            };

            let mut device = self.devices.get(&id).unwrap().borrow_mut();

            if let Err(e) = device.handle_response(response)
            {
                println!("Error received while device processed response {:?}", e);
            }
        }
    }

    fn try_send(&mut self)
    {
        if let Some(id) = self.scheduled_devices.pop_front()
        {
            let device = self.devices.get(&id).unwrap();

            let (request, has_more) = device.borrow_mut().next_request().unwrap();

            let (tx, rx) = oneshot::channel();

            self.tx.try_send((id, request, tx)).unwrap();

            self.pending_response = Some((id, rx));

            if has_more
            {
                self.scheduled_devices.push_front(id);
            }
        }
    }
}