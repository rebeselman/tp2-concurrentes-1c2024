use crate::order::Order;

use super::Message;

#[derive(Default)]
pub struct Abort {
    order: Order
}

impl Abort {
    pub fn new() -> Self {
        Abort {order: Order::default()}
    }
}

impl Message for Abort {
    fn process_message(&mut self) {
        todo!()
    }
    
    fn add_order(&mut self, order: Order) {
        self.order = order
    }
}