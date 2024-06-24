use super::item::Item;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Order {
    order_id: i32,
    client_id: i32,
    credit_card: String,
    items: Vec<Item>,
}
