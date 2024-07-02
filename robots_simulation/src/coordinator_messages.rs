//! Messages that the coordinator sends to the robots

use std::net::SocketAddr;
use actix::Message;
use orders::order::Order;
use orders::ice_cream_flavor::IceCreamFlavor;
use serde::{Deserialize, Serialize};


#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum CoordinatorMessage {
    AccessAllowed {
        flavor: IceCreamFlavor,
    },
    AccessDenied {
        reason: String,
    },
    OrderReceived {
        robot_id: usize,
        order: Order,
        screen_addr: SocketAddr
    },
    OrderAborted {
        robot_id: usize,
        order: Order
    },
    ACK,
}
