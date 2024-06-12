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
    fn process_message(&mut self) {
        todo!()
    }

    fn add_order(&mut self, order: Order) {
        self.order = order
    }
}