use abort::Abort;
use commit::Commit;
use prepare::Prepare;

use crate::order::Order;

pub mod abort;
pub mod commit;
pub mod prepare;

pub trait Message {
    fn process_message(&mut self) {}

    fn add_order(&mut self, _order: Order) {}
}

pub fn deserialize_message(string: String) -> Result<Box<dyn Message>, String> {
    let words: Vec<&str> = string.split_whitespace().collect();
    
    if words.len() < 4 {
        return Err("Incomplete message".to_owned());
    }
    
    let mut message: Box<dyn Message> = match words[0].trim() {
        "abort" => Box::new(Abort::new()),
        "commit" => Box::new(Commit::new()),
        "prepare" => Box::new(Prepare::new()),
        _ => return Err("Unknown message".to_owned()),
    };
    
    let order_id = words[1].trim().to_string();
    let client_id = words[2].trim().to_string();
    let credit_card_number = words[3].trim().to_string();

    let order = Order::new(
        order_id.clone(),
        client_id.clone(),
        credit_card_number.clone(),
    );

    message.add_order(order);
    Ok(message)
}
