//! Messages that robots send to the coordinator
//! access request, access release and order completed and commit i think?
use std::net::SocketAddr;

use actix::Message;
use orders::ice_cream_flavor::IceCreamFlavor;
use orders::order::Order;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub enum RobotResponse {
    AccessRequest {
        robot_id: usize,
        flavors: Vec<IceCreamFlavor>,
        addr: SocketAddr,
    },
    ReleaseRequest {
        robot_id: usize,
        flavor: IceCreamFlavor,
        addr: SocketAddr,
    },
    OrderFinished {
        robot_id: usize,
        order: Order,
    },
    OrderInProcess {
        robot_id: usize,
        order: Order,
        addr: SocketAddr,
        screen_addr: SocketAddr
    },
    ReassignOrder {
        robot_id: usize
    },
    NoOrderInProcess {
        robot_id: usize,
        addr: SocketAddr,
    },
}
