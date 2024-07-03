use super::abort::Abort;
use super::commit::Commit;
use super::prepare::Prepare;
use orders::order::Order;

/// Trait representing a generic message.
pub trait Message: Send + Sync {
    /// Returns a reference to the associated order.
    fn get_order(&self) -> &Order;

    /// Returns the message type as a string.
    fn type_to_string(&self) -> String;

    /// Returns the corresponding response type as a string.
    fn get_response_type(&self) -> String;

    /// Returns a vector of bytes representing the respond message.
    /// The format will be:
    /// {order_id}\n{message_type} -> DEPRECATED
    /// {message_type}\n{order_id}
    fn process(&self) -> Vec<u8> {
        //format!("{}\n{}", self.get_order().id(), self.get_response_type()).into_bytes()
        format!("{}\n{}", self.get_response_type(), self.get_order().id()).into_bytes()
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
/// The string format should be
/// `{message_type}\n{payload}`
/// with payload being the serialized Order.
///
/// # Errors
///
/// Returns an error if the message is incomplete, has an empty payload, or if JSON deserialization fails.
pub fn deserialize_message(message: String) -> Result<Box<dyn Message>, String> {
    if message.trim().is_empty() {
        return Err("Incomplete message: missing message type and payload".to_owned());
    }

    let mut parts = message.splitn(2, '\n');
    let message_type = parts
        .next()
        .ok_or_else(|| "Incomplete message: missing type or payload".to_owned())?;
    if message_type.trim().is_empty() {
        return Err("Incomplete message: empty type".to_owned());
    }
    let json_payload = parts
        .next()
        .ok_or_else(|| "Incomplete message: missing type or payload".to_owned())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_abort_message() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Abort::new(order);
        assert_eq!(message.process(), b"abort\n9")
    }

    #[test]
    fn test_process_commit_message() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Commit::new(order);
        assert_eq!(message.process(), b"finished\n9")
    }

    #[test]
    fn test_process_prepare_message() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Prepare::new(order);
        let result = message.process();
        assert!(result == b"ready\n9" || result == b"abort\n9");
    }

    #[test]
    fn test_generate_abort_log_entry() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Abort::new(order);
        let log_entry =
            r#"abort {"order_id":9,"client_id":25,"credit_card":"0000111122223333","items":[]}"#;
        assert_eq!(message.log_entry().unwrap(), format!("{}\n", log_entry))
    }

    #[test]
    fn test_generate_commit_log_entry() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Commit::new(order);
        let log_entry =
            r#"commit {"order_id":9,"client_id":25,"credit_card":"0000111122223333","items":[]}"#;
        assert_eq!(message.log_entry().unwrap(), format!("{}\n", log_entry))
    }

    #[test]
    fn test_generate_prepare_log_entry() {
        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Prepare::new(order);
        let log_entry =
            r#"prepare {"order_id":9,"client_id":25,"credit_card":"0000111122223333","items":[]}"#;
        assert_eq!(message.log_entry().unwrap(), format!("{}\n", log_entry))
    }

    #[test]
    fn test_deserialize_valid_abort_message() {
        let message = "abort\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}".to_string();
        let abort_msg = deserialize_message(message).unwrap();
        assert_eq!(abort_msg.type_to_string(), "abort");
        assert_eq!(abort_msg.get_order().id(), 9);
        assert_eq!(abort_msg.get_order().client_id(), 25);
        assert_eq!(
            abort_msg.get_order().credit_card(),
            "0000111122223333".to_string()
        );
        assert!(abort_msg.get_order().items().is_empty())
    }

    #[test]
    fn test_deserialize_valid_commit_message() {
        let message = "commit\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}".to_string();
        let abort_msg = deserialize_message(message).unwrap();
        assert_eq!(abort_msg.type_to_string(), "commit");
        assert_eq!(abort_msg.get_order().id(), 9);
        assert_eq!(abort_msg.get_order().client_id(), 25);
        assert_eq!(
            abort_msg.get_order().credit_card(),
            "0000111122223333".to_string()
        );
        assert!(abort_msg.get_order().items().is_empty())
    }

    #[test]
    fn test_deserialize_valid_prepare_message() {
        let message = "prepare\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}".to_string();
        let abort_msg = deserialize_message(message).unwrap();
        assert_eq!(abort_msg.type_to_string(), "prepare");
        assert_eq!(abort_msg.get_order().id(), 9);
        assert_eq!(abort_msg.get_order().client_id(), 25);
        assert_eq!(
            abort_msg.get_order().credit_card(),
            "0000111122223333".to_string()
        );
        assert!(abort_msg.get_order().items().is_empty())
    }

    #[test]
    fn test_deserialize_empty_message() {
        let message = "".to_string();
        match deserialize_message(message) {
            Err(err) => assert_eq!("Incomplete message: missing message type and payload", err),
            _ => panic!("Expected error not returned"),
        }
    }

    #[test]
    fn test_deserialize_message_missing_type() {
        let message =
            "{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}"
                .to_string();
        match deserialize_message(message) {
            Err(err) => assert_eq!("Incomplete message: missing type or payload", err),
            _ => panic!("Expected error not returned"),
        }
    }

    #[test]
    fn test_deserialize_message_missing_payload() {
        let message = "abort".to_string();
        match deserialize_message(message) {
            Err(err) => assert_eq!("Incomplete message: missing type or payload", err),
            _ => panic!("Expected error not returned"),
        }
    }

    #[test]
    fn test_deserialize_message_empty_type() {
        let message =
            "\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}"
                .to_string();
        match deserialize_message(message) {
            Err(err) => assert_eq!("Incomplete message: empty type", err),
            _ => panic!("Expected error not returned"),
        }
    }

    #[test]
    fn test_deserialize_message_empty_payload() {
        let message = "abort\n".to_string();
        match deserialize_message(message) {
            Err(err) => assert_eq!("Incomplete message: empty payload", err),
            _ => panic!("Expected error not returned"),
        }
    }

    #[test]
    fn test_deserialize_message_invalid_json() {
        let message = "abort\ninvalid_json".to_string();
        assert!(deserialize_message(message).is_err())
    }

    #[test]
    fn test_deserialize_unknown_message_type() {
        let message = "unknown\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}".to_string();
        match deserialize_message(message) {
            Err(err) => assert_eq!("Unknown message 'unknown'", err),
            _ => panic!("Expected error not returned"),
        }
    }
}
