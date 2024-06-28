//! Screen Messages
//! Messages that the screen sends to the coordinator
use std::net::SocketAddr;

use actix::Message;
use orders::order::Order;
use serde::{Deserialize, Serialize};

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum ScreenMessage {
    OrderRequest {
        order: Order,
        screen_addr: SocketAddr,
        
    },
    CommitReceived {
        order: Order,
    },
    Abort{
        order: Order,
    },

}