use std::net::SocketAddr;

use orders::order::Order;

use crate::order_status::OrderStatus;

#[derive(Clone)]
pub struct OrderState {
    pub order: Order,
    pub status: OrderStatus,
    pub screen_addr: SocketAddr,
    pub robot_id: Option<usize>,
}
