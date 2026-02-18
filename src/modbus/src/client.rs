use crate::{ Device, Result, rtu };
use crate::protocol::Header;

pub use rtu::ClientConfig as ClientConfigRtu;
pub use rtu::Client       as ClientRtu;

#[derive(Debug)]
pub enum Client
{
    Rtu(ClientRtu),
}

impl Client 
{
    pub fn rtu(config: ClientConfigRtu) -> tokio_serial::Result<Self>
    {   
        let client = rtu::Client::new(config)?;
        Ok(Self::Rtu(client))
    }

    pub fn device<'a>(&'a mut self, slave_id: u8) -> Device<'a> 
    {
        Device::new(self, slave_id)
    }

    pub(crate) async fn send_recv(
        &mut self, 
        header: Header,
        data:   &[u8]
    ) -> Result<&mut [u8]>
    {
        match self 
        {
            Client::Rtu(client) => 
            {
                client.send(&header, data).await?;
                client.recv(&header).await
            },
        }
    }
}