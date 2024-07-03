//! Types of states of an order of ice cream

use std::net::SocketAddr;
use std::time::Instant;
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]

pub enum OrderState {
    Wait(Instant), // esto indica que el pedido se esta procesando
    Finished, // esto indica que el pedido fue confirmado o sea completado (en esta fase no se puede abortar)
    Abort,    // esto indica que el pedido fue abortado
    Ready,    // esto indica que el pedido se puede preparar
    ChangingOrderManagement(SocketAddr), // esto indica que el coordinador cambió
}
