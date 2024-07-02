use super::message::Message;
use orders::order::Order;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abort_msg_created_correctly() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let abort_msg = Abort::new(order);
        assert_eq!(abort_msg.get_order().id(), 9);
        assert_eq!(abort_msg.get_order().client_id(), 25);
        assert_eq!(
            abort_msg.get_order().credit_card(),
            "0000111122223333".to_string()
        );
        assert!(abort_msg.get_order().items().is_empty())
    }
}
