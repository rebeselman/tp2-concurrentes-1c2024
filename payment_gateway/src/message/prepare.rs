use rand::Rng;

use crate::order::Order;

use super::Message;

const CAPTURE_PROBABILITY: f64 = 0.9;

pub struct Prepare {
    pub order: Order,
}

impl Prepare {
    pub fn new(order: Order) -> Self {
        Prepare { order }
    }
}

impl Message for Prepare {
    fn process(&self) -> Result<Vec<u8>, String> {
        let mut message;

        let captured = rand::thread_rng().gen_bool(CAPTURE_PROBABILITY);
        if captured {
            message = b"ready\n".to_vec();
        } else {
            message = b"abort\n".to_vec();
        }

        let order_serialized = serde_json::to_vec(&self.order).map_err(|e| e.to_string())?;
        message.extend_from_slice(&order_serialized);
        message.push(0u8);
        Ok(message)
    }

    fn type_to_string(&self) -> String {
        "prepare".to_string()
    }

    fn log_entry(&self) -> Result<String, String> {
        let order_serialized = serde_json::to_string(&self.order).map_err(|e| e.to_string())?;
        let log_entry = format!("{} {}\n", self.type_to_string(), order_serialized);
        Ok(log_entry)
    }
}
