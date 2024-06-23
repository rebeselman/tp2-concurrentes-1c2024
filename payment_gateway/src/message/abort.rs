use super::Message;
use crate::order::Order;

pub struct Abort {
    order: Order,
}

impl Abort {
    pub fn new(order: Order) -> Self {
        Abort { order }
    }
}

impl Message for Abort {
    fn process(&self) -> Result<Vec<u8>, String> {
        let mut message = b"abort\n".to_vec();

        let order_serialized = serde_json::to_vec(&self.order).map_err(|e| e.to_string())?;
        message.extend_from_slice(&order_serialized);
        message.push(0u8);
        Ok(message)
    }

    fn type_to_string(&self) -> String {
        "abort".to_string()
    }

    fn log_entry(&self) -> Result<String, String> {
        let order_serialized = serde_json::to_string(&self.order).map_err(|e| e.to_string())?;
        let log_entry = format!("{} {}\n", self.type_to_string(), order_serialized);
        Ok(log_entry)
    }
}
