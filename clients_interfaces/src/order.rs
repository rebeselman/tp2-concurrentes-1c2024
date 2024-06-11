//! Represents an order from a client asking for items offered by an ice cream local
use serde::{Deserialize, Serialize};

use crate::item::Item;
#[derive(Serialize, Deserialize)]
pub struct Order {
    order_id: u32,
    client_id: u32,
    credit_card: String,
    items: Vec<Item>

}



impl Order {
    /// To obtein the id of this order
    pub fn id(&self)-> u32{
        return self.order_id
    }
}