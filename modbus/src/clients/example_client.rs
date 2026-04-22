use tokio::sync::{mpsc, oneshot};
use tokio_modbus::{
    Slave,
    client::{Client, Context},
    slave::SlaveContext,
};

use crate::{ExceptionCode, Request, Response};

pub type ResponseMessage = Result<Response, ExceptionCode>;
pub type RequestMessage = (u8, Request, mpsc::Sender<ResponseMessage>);

pub struct ExampleClient;

impl ExampleClient {
    pub fn create_channels() -> (mpsc::Sender<RequestMessage>, mpsc::Receiver<RequestMessage>) {
        mpsc::channel(255)
    }

    pub async fn run(
        mut ctx: Context,
        mut rx: mpsc::Receiver<(u8, Request, mpsc::Sender<ResponseMessage>)>,
    ) {
        loop {
            let (slave_id, request, tx) = match rx.recv().await {
                Some(v) => v,
                None => break,
            };
            ctx.set_slave(Slave(slave_id));
            let now = std::time::Instant::now();
            match ctx.call(request).await {
                Ok(result) => {
                    //println!("took {} {:?}",std::time::Instant::now().duration_since(now).as_millis(),result);
                    let res = tx.send(result).await;
                    match res {
                        Ok(r) => r,
                        Err(e) => println!("error:{:?}", e),
                    }
                }
                Err(e) => println!("ex client error {:?}", e),
            }
        }
    }
}
