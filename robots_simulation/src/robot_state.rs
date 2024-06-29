use orders::{ice_cream_flavor::IceCreamFlavor, order::Order};

#[derive(Debug, Clone)]
pub enum RobotState {
    Idle,
    WaitingForAccess(Order, Vec<IceCreamFlavor>),
    ProcessingOrder(Order),
}
