use actix::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use orders::order::Order;

use orders::ice_cream_flavor::IceCreamFlavor;


#[derive(Serialize, Deserialize, Debug)]
pub enum RequestToCoordinator {
    SolicitarAcceso { robot_id: usize, flavors: Vec<IceCreamFlavor> },
    LiberarAcceso { robot_id: usize, flavor: IceCreamFlavor },
    OrdenTerminada { robot_id: usize, order: Order },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    AccesoConcedido(IceCreamFlavor),
    AccesoDenegado(String),
    ACK,
    AssignOrder { robot_id: usize, order: Order },
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AccessRequest {
    pub robot_id: usize,
    pub flavors: Vec<IceCreamFlavor>,
    pub addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReleaseRequest {
    pub robot_id: usize,
    pub flavor: IceCreamFlavor,
    pub addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OrderRequest {
    pub order: Order,
    pub screen_addr: SocketAddr,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderReceived {
    pub robot_id: usize,
    pub order: Order,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CommitReceived {
    pub robot_id: usize,
    pub order: Order,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OrderFinished {
    pub robot_id: usize,
    pub order: Order,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AccessAllowed {
    pub flavor: IceCreamFlavor,
}

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AccessDenied {
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum RobotState {
    Idle,
    WaitingForAccess(Order, Vec<IceCreamFlavor>),
    ProcessingOrder(Order),
}

#[derive(Clone)]
pub struct OrderState {
    pub order: Order,
    pub status: OrderStatus,
    pub screen_addr: SocketAddr,
}

#[derive(Clone, Eq, PartialEq)]
pub enum OrderStatus {
    Pending,
    CompletedButNotCommited,
    CommitReceived,
    Completed,
}