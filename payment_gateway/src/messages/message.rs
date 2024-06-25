use super::abort::Abort;
use super::commit::Commit;
use super::prepare::Prepare;
use orders::order::Order;

/// Trait representing a generic message.
pub trait Message {
    /// Returns a reference to the associated order.
    fn get_order(&self) -> &Order;

    /// Returns the message type as a string.
    fn type_to_string(&self) -> String;

    /// Returns the corresponding response type as a string.
    fn get_response_type(&self) -> String;

    /// Returns a vector of bytes representing the respond message.
    /// The format will be:
    /// {order_id}\n{message_type}
    fn process(&self) -> Vec<u8> {
        format!(
            "{}\n{}",
            self.get_order().order_id,
            self.get_response_type()
        )
        .into_bytes()
    }

    /// Generates a log entry for the message and returns it as a string.
    ///
    /// # Errors
    ///
    /// Returns an error string if serialization of the order fails.
    fn log_entry(&self) -> Result<String, String> {
        let order_serialized =
            serde_json::to_string(&self.get_order()).map_err(|e| e.to_string())?;
        let log_entry = format!("{} {}\n", self.type_to_string(), order_serialized);
        Ok(log_entry)
    }
}

/// Converts the message string to its correspondent object type.
/// The string format should be:
/// {message_type}\n{payload}\0
/// with payload being the serialized Order.
///
/// # Errors
///
/// Returns an error if the message is incomplete, has an empty payload, or if JSON deserialization fails.
pub fn deserialize_message(message: String) -> Result<Box<dyn Message>, String> {
    let mut parts = message.splitn(2, '\n');
    let message_type = parts
        .next()
        .ok_or_else(|| "Incomplete message: missing message type and payload".to_owned())?;
    let json_payload = parts
        .next()
        .ok_or_else(|| "Incomplete message: missing payload".to_owned())?;

    if json_payload.trim().is_empty() {
        return Err("Incomplete message: empty payload".to_owned());
    }

    let order: Order = serde_json::from_str(json_payload).map_err(|e| e.to_string())?;
    let message: Box<dyn Message> = match message_type {
        "abort" => Box::new(Abort::new(order)),
        "commit" => Box::new(Commit::new(order)),
        "prepare" => Box::new(Prepare::new(order)),
        _ => return Err(format!("Unknown message '{}'", message_type)),
    };

    Ok(message)
}
