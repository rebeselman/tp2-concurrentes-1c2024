use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum ElectionMessage {
    Election {
        robot_id: usize
    },
    NewCoordinator {
        robot_id: usize
    },
    Ok {
        robot_id: usize
    },
}