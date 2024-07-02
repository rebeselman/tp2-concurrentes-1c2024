//! This module contains the definition of the ScreenMessage enum.
//! A message type for communication between screens to check if they are still alive
//! and to exchange information about the last order processed. This would be used to
//! reassign orders from a screen that has crashed to another screen.
use actix::Message;

use serde::{Deserialize, Serialize};


#[derive(Copy, Clone, Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
/// A message type for communication between screens to check if they are still alive
/// Ping: A message sent by a screen to check if another screen is still alive
/// Pong: A message sent by a screen to respond to a Ping message with the last order completed
pub enum ScreenMessage {
    Ping { screen_id: usize },
    Pong { screen_id: usize, last_order: Option<usize>},
    Finished { screen_id: usize}
}
