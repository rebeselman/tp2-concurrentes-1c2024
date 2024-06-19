use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    pub container: ContainerType,
    pub units: u32,
    pub flavors: Vec<IceCreamFlavor>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum IceCreamFlavor {
    Chocolate,
    Strawberry,
    Vanella,
    Mint,
    Lemon,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ContainerType {
    Cup,
    Cone,
    OneKilo,
    HalfKilo,
    QuarterKilo,
}

#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
pub enum Request {
    SolicitarAcceso { robot_id: usize, gusto: String, robot_addr: String },
    LiberarAcceso { robot_id: usize, gusto: String, robot_addr: String},
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    AccesoConcedido,
    AccesoDenegado(String),
}
