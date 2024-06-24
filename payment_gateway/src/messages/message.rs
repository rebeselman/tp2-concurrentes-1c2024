use super::abort::Abort;
use super::commit::Commit;
use super::prepare::Prepare;
use crate::orders::order::Order;

/// Trait representing a generic message.
pub trait Message {
    /// Returns a reference to the associated order.
    fn get_order(&self) -> &Order;

    /// Returns the message type as a string.
    fn type_to_string(&self) -> String;

    /// Returns the corresponding response type as a string.
    fn get_response_type(&self) -> String;

    /// Returns a vector of bytes representing the respond message.
    ///
    /// # Errors
    ///
    /// Returns an error string if serialization of the order fails.
    fn process(&self) -> Result<Vec<u8>, String> {
        let message_type = self.get_response_type();
        let mut message = message_type.as_bytes().to_vec();
        message.push(b'\n');

        let order_serialized = serde_json::to_vec(&self.get_order()).map_err(|e| e.to_string())?;
        message.extend_from_slice(&order_serialized);
        message.push(0u8);
        Ok(message)
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
