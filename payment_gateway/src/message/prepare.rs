use std::net::SocketAddr;

use rand::Rng;
use std::net::UdpSocket;

use crate::order::Order;

use super::Message;

const CAPTURE_PROBABILITY: f64 = 0.9;

#[derive(Default)]
pub struct Prepare {
    order: Order,
}

impl Prepare {
    pub fn new() -> Self {
        Prepare {
            order: Order::default(),
        }
    }
}

impl Message for Prepare {
    fn process_message(&mut self, socket: &UdpSocket, addr: SocketAddr) {
        let order_serialized = serde_json::to_vec(&self.order).unwrap();
        let mut message;

        let captured = rand::thread_rng().gen_bool(CAPTURE_PROBABILITY);
        if captured {
            message = b"ready\n".to_vec();
        } else {
            message = b"abort\n".to_vec();
        }
        message.extend_from_slice(&order_serialized);
        message.push(0u8);

        let _ = socket.send_to(&message, addr);
    }

    fn add_order(&mut self, order: Order) {
        self.order = order
    }
}
