use std::net::SocketAddr;

use abort::Abort;
use commit::Commit;
use prepare::Prepare;
use std::net::UdpSocket;

use crate::order::Order;

pub mod abort;
pub mod commit;
pub mod prepare;

pub trait Message {
    fn process_message(&mut self, _socket: &UdpSocket, _addr: SocketAddr) {}

    fn add_order(&mut self, _order: Order) {}
}

pub fn deserialize_message(message: String) -> Result<Box<dyn Message>, String> {
    let mut parts = message.split('\n');
    
    if parts.clone().count() < 2 {
        return Err("Incomplete message".to_owned());
    }

    let message_type = parts.next();
    let mut message: Box<dyn Message> = match message_type {
        Some("abort") => Box::new(Abort::new()),
        Some("commit") => Box::new(Commit::new()),
        Some("prepare") => Box::new(Prepare::new()),
        _ => return Err("Unknown message".to_owned()),
    };
    
    let order: Order = serde_json::from_str(&parts.next().unwrap()).unwrap();

    message.add_order(order);
    Ok(message)
}
