use crate::order::Order;

use super::Message;

#[derive(Default)]
pub struct Commit {
    order: Order,
}

impl Commit {
    pub fn new() -> Self {
        Commit {
            order: Order::default(),
        }
    }
}

impl Message for Commit {
    fn process(&self) -> Vec<u8> {
        todo!()
    }

    fn add_order(&mut self, order: Order) {
        self.order = order
    }

    fn to_string(&self) -> String {
        "commit".to_string()
    }

    fn log(&self) -> Result<String, String> {
        let order_serialized = serde_json::to_string(&self.order).map_err(|e| e.to_string())?;
        let log_entry = format!("{} {}\n", self.to_string(), order_serialized);
        Ok(log_entry)
    }
}
