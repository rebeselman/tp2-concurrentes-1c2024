//! Represents a screen of a ice cream local

use std::{collections::HashMap, net::UdpSocket, sync::{Arc, Condvar, Mutex}};
use crate::{order_id::OrderId, order_state::OrderState};


pub struct Screen {
    log: HashMap<OrderId, OrderState>,
    socket: UdpSocket,
    responses: Arc<(Mutex<Vec<Option<OrderState>>>, Condvar)>
}
