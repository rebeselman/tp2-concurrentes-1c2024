use crate::order_state::OrderState;

#[derive(Debug)]
pub struct Order {
    order_id: String,
    client_id: String,
    credit_card_number: String,
    state: OrderState,
}

impl Order {
    pub fn new(order_id: String, client_id: String, credit_card_number: String) -> Self {
        Self {
            order_id,
            client_id,
            credit_card_number,
            state: OrderState::Pending,
        }
    }

    pub fn update_state(&mut self, new_state: OrderState) {
        self.state = new_state;
    }

    // fn log_state_change(&self, file: &mut std::fs::File) -> io::Result<()> {
    //     writeln!(file, "Order {:?} - State: {:?}", self.order_id, self.state)
    // }
}