use serde::{Deserialize, Serialize};
use tokio::time::Instant;

#[derive(Debug, Serialize, Deserialize)]
pub enum PingMessage {
    Ping,
    Pong,
}

#[derive(Debug, Clone)]
pub struct PeerStatus {
    pub(crate) last_pong: Option<Instant>,
    pub(crate) ping_attempts: usize,
    // pub is_alive: bool,
}