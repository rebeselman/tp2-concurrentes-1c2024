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
    items: Vec<Item>
}

impl Default for Order {
    fn default() -> Order {
        Self { order_id: -1, client_id: -1, credit_card_number: "".to_string(), items: Vec::new() }
    }
}