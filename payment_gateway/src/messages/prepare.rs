use super::message::Message;
use orders::order::Order;
use rand::Rng;

const CAPTURE_PROBABILITY: f64 = 0.9;

/// Represents a `Prepare` message with its corresponding order.
pub struct Prepare {
    pub order: Order,
}

impl Prepare {
    /// Creates a new `Prepare` message with the given order.
    pub fn new(order: Order) -> Self {
        Prepare { order }
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

    /// Returns the response type based on a capture probability.
    ///
    /// If a random number generated falls below the `CAPTURE_PROBABILITY`, the response
    /// type is "ready". Otherwise, it is "abort".
    fn get_response_type(&self) -> String {
        let captured = rand::thread_rng().gen_bool(CAPTURE_PROBABILITY);
        if captured {
            "ready".to_string()
        } else {
            "abort".to_string()
        }
    }
}
