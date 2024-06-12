use rand::Rng;

use crate::{order::Order, order_state::OrderState};

use super::Message;

const CAPTURE_PROBABILITY: f64 = 0.9;

#[derive(Default)]
pub struct Prepare {
    order: Order,
}

impl Prepare {
    pub fn new() -> Self {
        Prepare {
            order: Order::default(),
        }
    }
}

impl Message for Prepare {
    fn process_message(&mut self) {
        let captured = rand::thread_rng().gen_bool(CAPTURE_PROBABILITY);
        if captured {
            self.order.update_state(OrderState::Captured);
            // server.send_to(b"ready", &addr)?;
        } else {
            self.order.update_state(OrderState::Rejected);
            // server.send_to(b"abort", &addr)?;
        }
    }

    fn add_order(&mut self, order: Order) {
        self.order = order
    }
}
