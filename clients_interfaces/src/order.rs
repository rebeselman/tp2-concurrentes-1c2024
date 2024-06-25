//! Represents an order from a client asking for items offered by an ice cream local
use serde::{Deserialize, Serialize};

use crate::item::Item;
#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    order_id: usize,
    client_id: usize,
    credit_card: String,
    items: Vec<Item>

}



impl Order {
    /// Creates a new order
    /// # Arguments
    /// * `order_id` - A usize representing the id of the order
    /// * `client_id` - A usize representing the id of the client
    /// * `credit_card` - A String representing the credit card of the client
    /// * `items` - A Vec<Item> representing the items the client is asking for
    /// # Returns
    /// * An Order
    pub fn new(order_id: usize, client_id: usize, credit_card: String, items: Vec<Item>)-> Order {
        return Order {
            order_id,
            client_id,
            credit_card,
            items
        }
    }


    /// To obtain the id of this order
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