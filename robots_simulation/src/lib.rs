use std::collections::HashMap;
use std::net::SocketAddr;
use actix::Message;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    pub container: ContainerType,
    pub units: u32,
    pub flavors: Vec<IceCreamFlavor>,
    pub flavor_status: HashMap<IceCreamFlavor, bool>,
    pub is_completed: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Eq, PartialEq)]
pub enum IceCreamFlavor {
    Chocolate,
    Strawberry,
    Vanella,
    Mint,
    Lemon,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ContainerType {
    Cup,
    Cone,
    OneKilo,
    HalfKilo,
    QuarterKilo,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    SolicitarAcceso { robot_id: usize, gustos: Vec<IceCreamFlavor> },
    LiberarAcceso { robot_id: usize, gusto: IceCreamFlavor },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    AccesoConcedido(IceCreamFlavor),
    AccesoDenegado(String),
    ACK,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AccessRequest {
    pub robot_id: usize,
    pub gustos: Vec<IceCreamFlavor>,
    pub addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReleaseRequest {
    pub robot_id: usize,
    pub gusto: IceCreamFlavor,
    pub addr: SocketAddr,
}