use orders::ice_cream_flavor::IceCreamFlavor;

#[derive(Clone, Debug)]
pub enum RobotStateForCoordinator {
    Idle,
    Disconnected,
    Busy {
        order_id: usize,
    },
    UsingContainer {
        order_id: usize,
        flavor: IceCreamFlavor,
    },
}