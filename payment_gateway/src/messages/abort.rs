use super::message::Message;
use crate::orders::order::Order;

/// Represents an `Abort` message with its corresponding order.
pub struct Abort {
    order: Order,
}

impl Abort {
    /// Creates a new `Abort` message with the given order.
    pub fn new(order: Order) -> Self {
        Abort { order }
    }
}

impl Message for Abort {
    /// Returns a reference to the associated order.
    fn get_order(&self) -> &Order {
        &self.order
    }

    /// Returns the message type as a string.
    fn type_to_string(&self) -> String {
        "abort".to_string()
    }

    /// Returns the response type as a string.
    fn get_response_type(&self) -> String {
        "abort".to_string()
    }
}
