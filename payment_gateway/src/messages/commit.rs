use super::message::Message;
use orders::order::Order;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_msg_created_correctly() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let commit_msg = Commit::new(order);
        assert_eq!(commit_msg.get_order().id(), 9);
        assert_eq!(commit_msg.get_order().client_id(), 25);
        assert_eq!(
            commit_msg.get_order().credit_card(),
            "0000111122223333".to_string()
        );
        assert!(commit_msg.get_order().items().is_empty())
    }
}
