use crate::order_state::OrderState;

#[derive(Debug)]
pub struct Order {
    _order_id: i32,
    _client_id: i32,
    _credit_card_number: String,
    state: OrderState,
}

impl Order {
    pub fn new(order_id: String, client_id: String, credit_card_number: String) -> Self {
        Self {
            _order_id: order_id.parse::<i32>().unwrap(),
            _client_id: client_id.parse::<i32>().unwrap(),
            _credit_card_number: credit_card_number,
            state: OrderState::Pending,
        }
    }

    pub fn update_state(&mut self, new_state: OrderState) {
        self.state = new_state;
    }
}

impl Default for Order {
    fn default() -> Order {
        Self { _order_id: -1, _client_id: -1, _credit_card_number: "".to_string(), state: OrderState::Pending }
    }
}