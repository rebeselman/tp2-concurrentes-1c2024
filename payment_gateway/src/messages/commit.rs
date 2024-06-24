use super::message::Message;
use crate::orders::order::Order;

/// Represents a `Commit` message with its corresponding order.
pub struct Commit {
    order: Order,
}

impl Commit {
    /// Creates a new `Commit` message with the given order.
    pub fn new(order: Order) -> Self {
        Commit { order }
    }
}

impl Message for Commit {
    /// Returns a reference to the associated order.
    fn get_order(&self) -> &Order {
        &self.order
    }

    /// Returns the message type as a string.
    fn type_to_string(&self) -> String {
        "commit".to_string()
    }

    /// Returns the response type as a string.
    fn get_response_type(&self) -> String {
        "finished".to_string()
    }
}
