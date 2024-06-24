use super::item::Item;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Order {
    pub order_id: usize,
    client_id: usize,
    credit_card: String,
    items: Vec<Item>,
}
