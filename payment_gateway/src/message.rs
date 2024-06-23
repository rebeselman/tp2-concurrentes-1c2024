use abort::Abort;
use commit::Commit;
use prepare::Prepare;

use crate::order::Order;

pub mod abort;
pub mod commit;
pub mod prepare;

pub trait Message {
    fn process(&self) -> Result<Vec<u8>, String>;

    fn type_to_string(&self) -> String;

    fn log_entry(&self) -> Result<String, String>;
}

pub fn deserialize_message(message: String) -> Result<Box<dyn Message>, String> {
    let mut parts = message.splitn(2,'\n');
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
