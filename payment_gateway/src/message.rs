use abort::Abort;
use commit::Commit;
use prepare::Prepare;

use crate::order::Order;

pub mod abort;
pub mod commit;
pub mod prepare;

pub trait Message {
    fn process_message(&self) -> Vec<u8>;

    fn add_order(&mut self, order: Order);

    fn to_string(&self) -> String;
}

pub fn deserialize_message(message: String) -> Result<Box<dyn Message>, String> {
    let mut parts = message.split('\n');
    let message_type = parts.next().ok_or_else(|| "Incomplete message".to_owned())?;
    let json_payload = parts.next().ok_or_else(|| "Incomplete message".to_owned())?;

    let mut message: Box<dyn Message> = match message_type {
        "abort" => Box::new(Abort::new()),
        "commit" => Box::new(Commit::new()),
        "prepare" => Box::new(Prepare::new()),
        _ => return Err("Unknown message".to_owned()),
    };

    let order: Order = serde_json::from_str(json_payload).map_err(|e| e.to_string())?;
    message.add_order(order);

    Ok(message)
}
