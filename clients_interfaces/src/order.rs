//! Represents an order from a client asking for items offered by an ice cream local
use serde::{Deserialize, Serialize};

use crate::item::Item;
#[derive(Serialize, Deserialize)]
pub struct Order {
    order_id: usize,
    client_id: usize,
    credit_card: String,
    items: Vec<Item>

}



impl Order {
    /// To obtein the id of this order
    pub fn id(&self)-> usize {
        return self.order_id
    }


    pub fn client_id(&self)-> usize {
        return self.client_id
    }


    pub fn credit_card(&self)-> &str {
        return &self.credit_card
    }

    pub fn items(&self)-> &Vec<Item> {
        return &self.items
    }

    
}