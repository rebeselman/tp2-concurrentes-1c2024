use rand::Rng;

use crate::order::Order;

use super::Message;

const CAPTURE_PROBABILITY: f64 = 0.9;


#[derive(Default)]
pub struct Prepare {
    pub order: Order,
}

impl Prepare {
    pub fn new() -> Self {
        Prepare {
            order: Order::default(),
        }
    }
}

impl Message for Prepare {
    fn process(&self) -> Vec<u8> {
        let order_serialized = serde_json::to_vec(&self.order).unwrap();
        let mut message;

        let captured = rand::thread_rng().gen_bool(CAPTURE_PROBABILITY);
        if captured {
            message = b"ready\n".to_vec();
        } else {
            message = b"abort\n".to_vec();
        }

        message.extend_from_slice(&order_serialized);
        message.push(0u8);
        message
    }

    fn add_order(&mut self, order: Order) {
        self.order = order
    }

    fn to_string(&self) -> String {
        "prepare".to_string()
    }

    fn log(&self) -> Result<String, String> {
        let order_serialized = serde_json::to_string(&self.order).map_err(|e| e.to_string())?;
        let log_entry = format!("{} {}\n", self.to_string(), order_serialized);
        Ok(log_entry)
    }
}
