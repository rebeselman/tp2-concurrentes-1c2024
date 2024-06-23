use item::Item;
use serde::{Deserialize, Serialize};

pub mod container_type;
pub mod ice_cream_flavor;
pub mod item;

#[derive(Deserialize, Serialize, Debug)]
pub struct Order {
    order_id: i32,
    client_id: i32,
    credit_card_number: String,
    items: Vec<Item>,
}
