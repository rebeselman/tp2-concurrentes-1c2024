use std::collections::HashMap;
use orders::{ice_cream_flavor::IceCreamFlavor, order::Order};

#[derive(Debug, Clone, PartialEq)]
pub enum RobotState {
    Idle,
    WaitingForAccess(Order, HashMap<IceCreamFlavor, u32>),
    ProcessingOrder(Order),
    UsingContainer(IceCreamFlavor),
}
