//! Represents an order from a client asking for items offered by an ice cream local
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::ice_cream_flavor::IceCreamFlavor;
use crate::item::Item;
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Order {
    order_id: usize,
    client_id: usize,
    credit_card: String,
    items: Vec<Item>,
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
    pub fn new(order_id: usize, client_id: usize, credit_card: String, items: Vec<Item>) -> Order {
        Order {
            order_id,
            client_id,
            credit_card,
            items,
        }
    }

    /// To obtain the id of this order
    pub fn id(&self) -> usize {
        self.order_id
    }

    pub fn client_id(&self) -> usize {
        self.client_id
    }

    pub fn credit_card(&self) -> &str {
        &self.credit_card
    }

    pub fn items(&self) -> &Vec<Item> {
        &self.items
    }

    pub fn time_to_prepare(&self) -> u32 {
        return self.items.iter().map(|item| item.time_to_prepare()).sum();
    }

    pub fn amounts_for_all_flavors(&self) -> HashMap<IceCreamFlavor, u32> {
        let mut flavor_totals = HashMap::new();

        for item in &self.items {
            for (flavor, amount) in item.amount_per_flavor() {
                *flavor_totals.entry(flavor).or_insert(0) += amount;
            }
        }
        flavor_totals
    }

    pub fn amounts_for_flavor(&self, flavor: IceCreamFlavor) -> u32 {
        self.amounts_for_all_flavors().get(&flavor).copied().unwrap_or(0)
    }
}
