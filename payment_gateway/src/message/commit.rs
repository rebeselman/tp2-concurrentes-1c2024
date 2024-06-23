use crate::order::Order;

use super::Message;

#[derive(Default)]
pub struct Commit {
    order: Order
}

impl Commit {
    pub fn new() -> Self {
        Commit {order: Order::default()}
    }
}

impl Message for Commit {
    fn process_message(&self) -> Vec<u8> {
        todo!()
    }

    fn add_order(&mut self, order: Order) {
        self.order = order
    }

    fn to_string(&self) -> String {
        "commit".to_string()
    }
}