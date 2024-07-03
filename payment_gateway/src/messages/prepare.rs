use super::message::Message;
use orders::order::Order;
use rand::{rngs::ThreadRng, Rng};

const CAPTURE_PROBABILITY: f64 = 0.5;

/// Represents a `Prepare` message with its corresponding order.
pub struct Prepare {
    pub order: Order,
}

impl Prepare {
    /// Creates a new `Prepare` message with the given order.
    pub fn new(order: Order) -> Self {
        Prepare { order }
    }

    /// Returns the response type based on a capture probability.
    ///
    /// If a random number generated falls below the `CAPTURE_PROBABILITY`, the response
    /// type is "ready". Otherwise, it is "abort".
    fn get_response_type(&self, rng: &mut dyn RandomNumberGenerator) -> String {
        let captured = rng.generate_bool(CAPTURE_PROBABILITY);
        if captured {
            "ready".to_string()
        } else {
            "abort".to_string()
        }
    }
}

impl Message for Prepare {
    /// Returns a reference to the associated order.
    fn get_order(&self) -> &Order {
        &self.order
    }

    /// Returns the message type as a string.
    fn type_to_string(&self) -> String {
        "prepare".to_string()
    }

    /// Wrapper for the actual function.
    fn get_response_type(&self) -> String {
        self.get_response_type(&mut rand::thread_rng())
    }
}

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
trait RandomNumberGenerator {
    fn generate_bool(&mut self, probability: f64) -> bool;
}

impl RandomNumberGenerator for ThreadRng {
    fn generate_bool(&mut self, probability: f64) -> bool {
        self.gen_bool(probability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_msg_created_correctly() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let prepare_msg = Prepare::new(order);
        assert_eq!(prepare_msg.get_order().id(), 9);
        assert_eq!(prepare_msg.get_order().client_id(), 25);
        assert_eq!(
            prepare_msg.get_order().credit_card(),
            "0000111122223333".to_string()
        );
        assert!(prepare_msg.get_order().items().is_empty())
    }

    #[test]
    fn test_payment_captured() {
        let prepare_msg = Prepare::new(Order::default());
        let mut mock = MockRandomNumberGenerator::new();
        mock.expect_generate_bool().returning(|_| true);
        assert_eq!(
            prepare_msg.get_response_type(&mut mock),
            "ready".to_string()
        )
    }

    #[test]
    fn test_payment_not_captured() {
        let prepare_msg = Prepare::new(Order::default());
        let mut mock = MockRandomNumberGenerator::new();
        mock.expect_generate_bool().returning(|_| false);
        assert_eq!(
            prepare_msg.get_response_type(&mut mock),
            "abort".to_string()
        )
    }
}
