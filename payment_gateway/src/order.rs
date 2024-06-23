use item::Item;
use serde::{Deserialize, Serialize};

pub mod container_type;
pub mod ice_cream_flavor;
pub mod item;

#[derive(Deserialize, Serialize, Debug)]
pub struct Order {
    _order_id: i32,
    _client_id: i32,
    _credit_card_number: String,
    _items: Vec<Item>
}

impl Default for Order {
    fn default() -> Order {
        Self { _order_id: -1, _client_id: -1, _credit_card_number: "".to_string(), _items: Vec::new() }
    }
}