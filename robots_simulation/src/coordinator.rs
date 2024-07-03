//! Coordinator module
//! This module contains the implementation of the Coordinator actor, which is responsible for managing the access to the ice cream containers and assigning orders to the robots.

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;

use actix::{Actor, Context, Handler};
use orders::ice_cream_flavor::IceCreamFlavor;
use orders::order::Order;
use serde_json::from_str;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use crate::container::Container;
use crate::robot_state_for_coordinator::RobotStateForCoordinator;

use super::coordinator_messages::CoordinatorMessage::{self, AccessAllowed, AccessDenied, OrderReceived};
use super::order_status::OrderStatus::{CommitReceived, Completed, CompletedButNotCommited, Pending};
use super::order_status_screen::OrderState;
use super::robot_messages::RobotResponse;
use super::screen_message::ScreenMessage;

#[derive(Clone)]

/// Coordinator
/// This struct represents the Coordinator actor, which is responsible for managing the access to the ice cream containers and assigning orders to the robots.
/// It contains the following fields:
/// * containers: HashMap<IceCreamFlavor, Arc<Mutex<bool>>> - A map of ice cream flavors to their respective container access state.
/// * socket: Arc<UdpSocket> - The UDP socket used to communicate with the robots and the screen.
/// * flavor_requests: Arc<Mutex<VecDeque<(Vec<IceCreamFlavor>, usize, SocketAddr)>> - A queue of access requests from the robots.
/// * order_queue: Arc<Mutex<VecDeque<Order>> - A queue of orders waiting to be assigned to a robot.
/// * robot_states: Arc<Mutex<HashMap<usize, bool>> - A map of robot IDs to their respective state (busy or available). false means available, true means busy.
/// * orders: HashMap<usize, OrderState> - A map of order IDs to their respective state.
pub struct Coordinator {
    containers: HashMap<IceCreamFlavor, Arc<Mutex<Container>>>,
    socket: Arc<UdpSocket>,
    order_queue: Arc<Mutex<VecDeque<(Order, SocketAddr)>>>,
    robot_states: HashMap<usize, Arc<Mutex<RobotStateForCoordinator>>>,
    orders: HashMap<usize, Arc<Mutex<OrderState>>>,
    received_all_updated_orders: Vec<usize>,
}

const NUMBER_ROBOTS: usize = 5;
const INITIAL_QUANTITY: u32 = 10000; // Initial quantity for each flavor

impl Coordinator {
    /// Creates a new Coordinator actor
    /// # Arguments
    /// * `socket` - An Arc<UdpSocket> representing the UDP socket used to communicate with the robots and the screen.
    /// * `coord_id` - The unique identifier for the coordinator to exclude from robot IDs.
    pub fn new(socket: Arc<UdpSocket>, coord_id: usize) -> Self {
        let flavors = vec![
            IceCreamFlavor::Chocolate,
            IceCreamFlavor::Strawberry,
            IceCreamFlavor::Vanilla,
            IceCreamFlavor::Mint,
            IceCreamFlavor::Lemon,
        ];

        let containers = flavors
            .into_iter()
            .map(|flavor| (flavor, Arc::new(Mutex::new(Container::new(INITIAL_QUANTITY)))))
            .collect();

        let robot_ids: Vec<usize> = (0..NUMBER_ROBOTS).filter(|&id| id != coord_id).collect();

        let robot_states = robot_ids.into_iter().map(|id| (id, Arc::new(Mutex::new(RobotStateForCoordinator::Idle)))).collect();

        Coordinator {
            containers,
            socket,
            order_queue: Arc::new(Default::default()),
            robot_states,
            orders: HashMap::new(),
            received_all_updated_orders: Vec::new(),
        }
    }

    /// Assigns an order to a robot

    async fn assign_order_to_robot(&mut self, order: Order, screen_addr: &SocketAddr) {
        for (&robot_id, state) in &self.robot_states {
            let mut state = state.lock().await;
            if matches!(*state, RobotStateForCoordinator::Idle) {
                *state = RobotStateForCoordinator::Busy { order_id: order.id() };

                // add robot id to order state
                if let Some(order_state) = self.orders.get_mut(&order.id()) {
                    let mut order_state = order_state.lock().await;
                    order_state.robot_id = Some(robot_id);
                }

                let robot_port: u16 = from_str::<u16>(format!("809{}", robot_id).as_str()).expect("Error parsing port");
                let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], robot_port));
                send_response(&self.socket, &OrderReceived { robot_id, order, screen_addr: *screen_addr }, addr)
                .await
                ;
                println!("[COORDINATOR] Order assigned to robot {}", robot_id);
                return;
            }
        }

        println!("[COORDINATOR] All robots are busy");
        // All robots are busy, handle accordingly (e.g., add to a queue)
        self.order_queue.lock().await.push_back((order, *screen_addr));
    }

    /// Marks an order as completed
    /// # Arguments
    /// * `order_id` - A usize representing the ID of the order to mark as completed
    async fn order_completed(&mut self, order_id: usize) {
        println!("[COORDINATOR] Processing completed order {}", order_id);
        if let Some(order_state) = self.orders.get_mut(&order_id) {
            let mut order_state = order_state.lock().await;
            if order_state.status == CommitReceived {
                order_state.status = Completed;
            } else if order_state.status == Pending {
                order_state.status = CompletedButNotCommited;
            }
        }
        // Check queue for pending orders
        let result = self.order_queue.lock().await.pop_front();
        if let Some((order, screen_addr)) = result {
            self.assign_order_to_robot(order, &screen_addr).await;
        }
    }

    async fn check_robot_has_container(&self, robot_id: usize, addr: SocketAddr) -> bool {
        let robot_state = self.robot_states.get(&robot_id).unwrap().clone();
        let robot_state = robot_state.lock().await;
        if let RobotStateForCoordinator::UsingContainer{ order_id: _, flavor } = *robot_state {
            let response = AccessAllowed { flavor };
            send_response(&self.socket, &response, addr).await;
            return true;
        }
        false
    }

    /// Checks if a flavor is available and sends a response to the robot
    async fn check_if_flavor_available(
        &mut self,
        robot_id: usize,
        flavors: &HashMap<IceCreamFlavor, u32>,
        addr: SocketAddr,
    ) -> bool {
        if self.check_robot_has_container(robot_id, addr).await {
            return true;
        }
        for (flavor, amount) in flavors {
            let container = self.containers.get(flavor).unwrap().clone();
            let mut container_state = container.lock().await;
            if container_state.is_available() {
                println!("[COORDINATOR] Robot {} is requesting access to container {:?}", robot_id, flavor);
                if !self.update_robot_state_to_using_container(&robot_id, flavor).await {
                    println!("[COORDINATOR] Robot {} isn't processing an order", robot_id);
                    return false;
                }
                if container_state.quantity() < *amount {
                    println!("[COORDINATOR] Container {:?} is not enough for robot {}", flavor, robot_id);
                    let robot_state = self.robot_states.get(&robot_id).unwrap().clone();
                    let robot_state = robot_state.lock().await;
                    if let RobotStateForCoordinator::UsingContainer { order_id, .. } = *robot_state {
                        self.abort_order_by_id(order_id).await;
                    };
                    return false;
                }
                container_state.use_container(robot_id, amount);
                println!("[COORDINATOR] Robot {} has access to container {:?}", robot_id, flavor);
                let response = AccessAllowed { flavor: *flavor };
                send_response(&self.socket, &response, addr).await;
                return true;
            }
            println!("[COORDINATOR] Container {:?} is not available for robot {}", flavor, robot_id);
        }
        false
    }

    async fn update_robot_state_to_using_container(&self, robot_id: &usize, flavor: &IceCreamFlavor) -> bool {
        let robot_state = self.robot_states.get(&robot_id).unwrap().clone();
        let mut robot_state = robot_state.lock().await;
        if let RobotStateForCoordinator::Busy { order_id } = *robot_state {
            *robot_state = RobotStateForCoordinator::UsingContainer { order_id, flavor: *flavor };
            return true;
        }
        false
    }

    async fn send_denied_access_to_robot(&self, addr: SocketAddr) {
        let response = AccessDenied { reason: "All requested containers are in use or empty".into() };
        send_response(&self.socket, &response, addr).await;
    }

    fn release_access_to_flavor(&mut self, robot_id: usize, flavor: &IceCreamFlavor) {
        let container = self.containers.get(flavor).unwrap().clone();
        let this = self.clone();
        println!("[COORDINATOR] Releasing access for container {:?} from {:?}", flavor, robot_id);
        actix_rt::spawn(async move {
            let mut container_state = container.lock().await;
            container_state.release_container();
            let robot_state = this.robot_states.get(&robot_id).unwrap().clone();
            let mut robot_state = robot_state.lock().await;
            if let RobotStateForCoordinator::UsingContainer { order_id, .. } = *robot_state {
                *robot_state = RobotStateForCoordinator::Busy { order_id };
            }
        });
    }


    /// Sends a finish message to the screen
    fn send_finish_message(&self, order_id: usize, addr: &SocketAddr) {
        let socket = self.socket.clone();
        let addr = *addr;
        println!("Sending finish message to screen {}", addr);
        actix_rt::spawn(async move {
            let message = format!("finished\n{}", order_id).into_bytes();
            socket.send_to(&message, &addr).await.unwrap();
        });
    }

    /// Send abort message to the screen
    fn send_abort_message(&self, order_id: usize, addr: &SocketAddr) {
        let socket = self.socket.clone();
        let addr = *addr;
        actix_rt::spawn(async move {
            let message = format!("abort\n{}", order_id).into_bytes();
            socket.send_to(&message, &addr).await.unwrap();
        });
    }

    async fn commit_received(&mut self, order: &Order) {
        let mut send_finished = false;
        let mut addr: SocketAddr = SocketAddr::new([0, 0, 0, 0].into(), 0);
        if let Some(order_state) = self.orders.get_mut(&order.id()) {
            let mut order_state = order_state.lock().await;
            if order_state.status == Pending {
                println!("[COORDINATOR] Received commit message for order: {}", order.id());
                order_state.status = CommitReceived;
            } else if order_state.status == CompletedButNotCommited {
                println!("[COORDINATOR] Received commit message for order: {}", order.id());
                order_state.status = Completed;
                send_finished = true;
                addr = order_state.screen_addr;
            }
        }
        // If address is not null
        if send_finished && addr != SocketAddr::new([0, 0, 0, 0].into(), 0) {
            println!("[COORDINATOR] Order completed: {}", order.id());
            self.send_finish_message(order.id(), &addr);
        }
    }

    fn abort_order(&mut self, order: Order) {
        println!("[COORDINATOR] Order aborted: {}", order.id());
        // remove the order from the orders
        if let Some(order_state) = self.orders.remove(&order.id()) {
            // stop the robot?
            let mut this = self.clone();
            actix_rt::spawn(async move {
                let order_state = order_state.lock().await;
                // if some robot was assigned to the order
                if let Some(robot_id) = order_state.robot_id {
                    this.send_abort_message_to_robot(order, robot_id).await;
                    // change my states as coordinator
                    this.free_robot_after_abort(robot_id).await;
                } else {
                    // if no robot was assigned to the order
                    // remove the order from the order queue
                    let mut order_queue = this.order_queue.lock().await;
                    order_queue.retain(|(o,_)| o.id() != order.id());
                }
                // send abort message to the screen
                let addr: SocketAddr = SocketAddr::new(order_state.screen_addr.ip(), order_state.screen_addr.port());
                this.send_abort_message(order_state.order.id(), &addr);
            });
        }
    }

    async fn abort_order_by_id(&mut self, order_id: usize) {
        let order = self.orders.get(&order_id).unwrap().clone();
        let order = order.lock().await.order.clone();
        self.abort_order(order);
    }

    async fn free_robot_after_abort(&mut self, robot_id: usize) {
        let robot_state = self.robot_states.get(&robot_id).unwrap().clone();
        let mut robot_state = robot_state.lock().await;
        println!("Robot state: {:?}", *robot_state);
        if let RobotStateForCoordinator::UsingContainer { flavor, .. } = *robot_state {
            println!("Releasing access to container {:?} from robot {}", flavor, robot_id);
            self.release_access_to_flavor(robot_id, &flavor);
        }
        *robot_state = RobotStateForCoordinator::Idle;
    }

    async fn send_abort_message_to_robot(&self, order: Order, robot_id: usize) {
        // send abort message to the robot
        println!("[COORDINATOR] Sending abort message to robot {}", robot_id);
        let msg = CoordinatorMessage::OrderAborted { robot_id, order };
        let robot_port: u16 = from_str::<u16>(format!("809{}", robot_id).as_str()).expect("Error parsing port");
        let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], robot_port));
        send_response(&self.socket, &msg, addr).await;
    }

    async fn reassign_order(&self, order: Order) {
        println!("[COORDINATOR] Reassigning order: {}", order.id());
        let order_id = order.id();
        if let Some(order_state) = self.orders.get(&order_id) {
            let order_state = order_state.lock().await;
            if order_state.status == Pending || order_state.status == CommitReceived {
                let mut this = self.clone();
                let addr = order_state.screen_addr;
                actix_rt::spawn(async move {
                    this.send_ready_message(&order, &addr).await;
                    this.assign_order_to_robot(order, &addr).await;
                });
            }
        }
    }

    fn register_order(&mut self, screen_addr: SocketAddr, order: &Order) {
        self.orders.insert(order.id(), Arc::new(Mutex::new(OrderState {
            order: order.clone(),
            status: Pending,
            screen_addr,
            robot_id: None,
        })));
    }

    async fn fix_order(&mut self, robot_id: usize) {
        println!("[COORDINATOR] Reassigning order for robot {}", robot_id);
        let robot_state = self.robot_states.get(&robot_id).unwrap().clone();
        let mut robot_state = robot_state.lock().await;
        match *robot_state {
            RobotStateForCoordinator::Busy { order_id } => {
                let order_state = self.orders.get(&order_id).unwrap();
                let order = order_state.lock().await.order.clone();
                self.reassign_order(order).await;
            }
            RobotStateForCoordinator::UsingContainer { order_id, flavor } => {
                self.release_access_to_flavor(robot_id, &flavor);
                let order_state = self.orders.get(&order_id).unwrap();
                let order = order_state.lock().await.order.clone();
                self.reassign_order(order).await;
            }
            _ => {}
        }
        *robot_state = RobotStateForCoordinator::Disconnected
    }

    async fn send_ready_message(&mut self, order: &Order, addr: &SocketAddr) {
        let message = format!("ready\n{}", order.id()).into_bytes();
        self.socket.send_to(&message, &addr).await.unwrap();
    }
}


/// Sends a response to a given address
async fn send_response(socket: &Arc<UdpSocket>, response: &CoordinatorMessage, addr: SocketAddr) {
    let mut message: Vec<u8> = b"order\n".to_vec();
    let request_serialized = serde_json::to_vec(response).unwrap();
    message.extend_from_slice(&request_serialized);
    socket.send_to(&message, addr).await.unwrap();
}

impl Actor for Coordinator {
    type Context = Context<Self>;
}

impl Handler<ScreenMessage> for Coordinator {
    type Result = ();

    /// Handles a ScreenMessage
    /// It sends an ACK message to the screen
    fn handle(&mut self, msg: ScreenMessage, _ctx: &mut Self::Context) {
        match msg {
            ScreenMessage::OrderRequest { order, screen_addr } => {
                self.register_order(screen_addr, &order);
                let order = order.clone();
                let addr = screen_addr;
                let mut this = self.clone();
                actix_rt::spawn(async move {
                    this.send_ready_message(&order, &addr).await;
                    this.assign_order_to_robot(order, &addr).await;
                });
            }
            ScreenMessage::CommitReceived { order } => {
                let order = order.clone();
                let mut this = self.clone();
                actix_rt::spawn(async move {
                    this.commit_received(&order).await;
                });
            }
            ScreenMessage::Abort { order } => {
                self.abort_order(order);
            }
        }
    }
}




impl Handler<RobotResponse> for Coordinator {
    type Result = ();
    fn handle(&mut self, msg: RobotResponse, _ctx: &mut Self::Context) -> Self::Result {

        match msg {
            RobotResponse::AccessRequest {
                robot_id,
                flavors,
                addr,
            } => {
                let flavors = flavors.clone();
                let mut this = self.clone();
                actix_rt::spawn(async move {
                    let access_given = this.check_if_flavor_available(robot_id, &flavors, addr)
                        .await;
                    if !access_given {
                        this.send_denied_access_to_robot(addr).await;
                    }
                });
            }
            RobotResponse::ReleaseRequest {
                robot_id,
                flavor,
                addr,
            } => {
                self.release_access_to_flavor(robot_id, &flavor);
                let socket = self.socket.clone();
                actix_rt::spawn(async move {
                    send_response(&socket, &CoordinatorMessage::ACK, addr).await;
                });

            }
            RobotResponse::OrderFinished { robot_id, order } => {
                println!("[COORDINATOR] Robot {} finished order: {}", robot_id, order.id());
                let order_id = order.id();
                let mut this = self.clone();
                actix_rt::spawn(async move {
                    this.order_completed(order_id).await;
                    if let Some(order_state) = this.orders.get(&order_id) {
                        let order_state = order_state.lock().await;
                        if order_state.status == Completed {
                            println!("[COORDINATOR] Order {} has Completed state", order_id);
                            this.send_finish_message(order_id, &order_state.screen_addr);
                        }
                        let robot_state = this.robot_states.get(&robot_id).unwrap().clone();
                        let mut robot_state = robot_state.lock().await;
                        *robot_state = RobotStateForCoordinator::Idle;
                    }
                });
            }
            RobotResponse::OrderInProcess { robot_id, order, addr: _addr, screen_addr } => {
                println!("[COORDINATOR] Registering order in process {} from robot {}", order.id(), robot_id);
                self.register_order(screen_addr, &order);
                let robot = self.robot_states.get(&robot_id);
                match robot {
                    Some(state) => {
                        self.received_all_updated_orders.push(robot_id);
                        let mut this = self.clone();
                        let state = state.clone();
                        actix_rt::spawn( async move {
                            this.send_ready_message(&order, &screen_addr).await;
                            let mut robot_state = state.lock().await;
                            *robot_state = RobotStateForCoordinator::Busy { order_id: order.id() };
                        });
                    }
                    None => {
                        // Check if I received all updated orders from robots
                        if self.received_all_updated_orders.len() == NUMBER_ROBOTS - 1 {
                            println!("All robots have updated orders");
                            self.received_all_updated_orders.clear();
                            let mut this = self.clone();
                            actix_rt::spawn( async move {
                                this.send_ready_message(&order, &screen_addr).await;
                                this.assign_order_to_robot(order, &screen_addr).await;
                            });
                        } else {
                            // Save order in queue
                            println!("Robot {} is not connected", robot_id);
                            let mut this = self.clone();
                            actix_rt::spawn( async move {
                                this.send_ready_message(&order, &screen_addr).await;
                                let mut order_queue = this.order_queue.lock().await;
                                order_queue.push_back((order, screen_addr));
                            });
                        }
                    }
                }
            }
            RobotResponse::ReassignOrder { robot_id } => {
                let mut this = self.clone();
                actix_rt::spawn(async move {
                    this.fix_order(robot_id).await;
                });
            }
            RobotResponse::NoOrderInProcess {
                robot_id,
                addr: _addr,
            } => {
                println!("No order in process for robot {}", robot_id);
                self.received_all_updated_orders.push(robot_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::net::{IpAddr, Ipv4Addr};
    use orders::container_type::ContainerType;
    use orders::item::Item;
    use rand::distributions::Alphanumeric;
    use rand::prelude::IndexedRandom;
    use rand::Rng;
    use tokio::net::UdpSocket;
    use tokio::sync::Mutex as AsyncMutex;

    use super::*;

    // Helper function to create a mock UdpSocket bound to an arbitrary available port
    async fn create_mock_socket() -> Arc<UdpSocket> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.expect("Failed to bind to address");
        Arc::new(socket)
    }

    // Helper function to create a Coordinator with a mock socket
    async fn setup_coordinator() -> Coordinator {
        let socket = create_mock_socket().await;
        Coordinator::new(socket, 999)
    }

    fn create_order() -> Order {
        let mut rng = rand::thread_rng();
        let order_id = rng.gen_range(0..1000);
        let client_id = rng.gen_range(0..1000);
        let credit_card: String = (0..16).map(|_| rng.sample(Alphanumeric) as char).collect();
        let mut items = Vec::new();
        for _ in 0..rng.gen_range(1..10) {
            // choose a random container

            let container: ContainerType = ContainerType::values()
                .choose(&mut rng)
                .cloned() // Clone the value to get an Option<ContainerType> instead of Option<&ContainerType>
                .ok_or_else(|| String::from("Error choosing random container")).unwrap();
            let units = rng.gen_range(1..5);
            let number_of_flavors = rng.gen_range(1..3);
            // vector of ice cream flavors
            let flavors: Vec<IceCreamFlavor> = (0..number_of_flavors)
                .map(|_| {
                    IceCreamFlavor::values()
                        .choose(&mut rng)
                        .ok_or_else(|| String::from("Error choosing flavors"))
                        .copied()
                })
                .collect::<Result<Vec<IceCreamFlavor>, String>>().unwrap_or_else(|_| vec![]);

            items.push(Item::new(container, units, flavors));
        }
        Order::new(order_id, client_id, credit_card, items)
    }

    #[actix_rt::test]
    async fn test_new_coordinator() {
        let coordinator = setup_coordinator().await;
        let orders = coordinator.orders.clone();
        let robot_states = coordinator.robot_states.clone();
        assert_eq!(orders.len(), 0);
        assert_eq!(robot_states.len(), 5);
    }

    #[actix_rt::test]
    async fn test_register_order() {
        let mut coordinator = setup_coordinator().await;
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();
        let id = order.id();

        coordinator.register_order(screen_addr, &order);
        let orders = coordinator.orders.clone();
        assert_eq!(orders.len(), 1);
        assert!(orders.contains_key(&id));
    }

    #[actix_rt::test]
    async fn test_free_robot_after_abort() {
        let mut coordinator = setup_coordinator().await;
        let robot_id = 1;
        coordinator.robot_states.insert(robot_id, Arc::new(AsyncMutex::new(RobotStateForCoordinator::UsingContainer { order_id: 1, flavor: IceCreamFlavor::Vanilla }))); // Assuming IceCreamFlavor::Vanilla exists

        coordinator.free_robot_after_abort(robot_id).await;

        let robot_state = coordinator.robot_states.get(&robot_id).unwrap().lock().await;
        matches!(*robot_state, RobotStateForCoordinator::Idle);
    }

    #[actix_rt::test]
    async fn test_assign_order_to_robot() {
        let mut coordinator = setup_coordinator().await;
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();
        coordinator.register_order(screen_addr, &order);

        coordinator.assign_order_to_robot(order.clone(), &screen_addr).await;

        let order_state = coordinator.orders.get(&order.id()).unwrap().lock().await;
        assert_eq!(order_state.status, Pending);
        assert!(order_state.robot_id.is_some());
    }

    #[actix_rt::test]
    async fn test_order_completed() {
        let mut coordinator = setup_coordinator().await;
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();
        coordinator.register_order(screen_addr, &order);

        coordinator.order_completed(order.id()).await;

        let order_state = coordinator.orders.get(&order.id()).unwrap().lock().await;
        assert_eq!(order_state.status, CompletedButNotCommited);
    }

    #[actix_rt::test]
    async fn test_check_if_flavor_available() {
        let mut coordinator = setup_coordinator().await;
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let robot_id = 1;
        let flavors = vec![IceCreamFlavor::Vanilla];
        let flavors = flavors.into_iter().map(|flavor| (flavor, 1)).collect();

        let access_granted = coordinator.check_if_flavor_available(robot_id, &flavors, addr).await;

        assert!(access_granted);
    }

    #[actix_rt::test]
    async fn test_send_denied_access_to_robot() {
        let coordinator = setup_coordinator().await;
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

        coordinator.send_denied_access_to_robot(addr).await;

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_release_access_to_flavor() {
        let mut coordinator = setup_coordinator().await;
        let robot_id = 1;
        let flavor = IceCreamFlavor::Vanilla;

        coordinator.release_access_to_flavor(robot_id, &flavor);

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_send_finish_message() {
        let coordinator = setup_coordinator().await;
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let order_id = 1;

        coordinator.send_finish_message(order_id, &addr);

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_commit_received() {
        let mut coordinator = setup_coordinator().await;
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();
        coordinator.register_order(screen_addr, &order);

        coordinator.commit_received(&order).await;

        let order_state = coordinator.orders.get(&order.id()).unwrap().lock().await;
        assert_eq!(order_state.status, CommitReceived);
    }

    #[actix_rt::test]
    async fn test_abort_order() {
        let mut coordinator = setup_coordinator().await;
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();
        coordinator.register_order(screen_addr, &order);

        coordinator.abort_order(order.clone());

        assert!(!coordinator.orders.contains_key(&order.id()));
    }

    #[actix_rt::test]
    async fn test_reassign_order() {
        let mut coordinator = setup_coordinator().await;
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();
        coordinator.register_order(screen_addr, &order);

        coordinator.reassign_order(order.clone()).await;

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_fix_order() {
        let mut coordinator = setup_coordinator().await;
        let robot_id = 1;
        let order = create_order();
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        coordinator.robot_states.insert(robot_id, Arc::new(AsyncMutex::new(RobotStateForCoordinator::Busy { order_id: order.id() })));
        coordinator.register_order(screen_addr, &order);

        coordinator.fix_order(robot_id).await;

        let robot_state = coordinator.robot_states.get(&robot_id).unwrap().lock().await;
        matches!(*robot_state, RobotStateForCoordinator::Disconnected);
    }

    #[actix_rt::test]
    async fn test_send_ready_message() {
        let mut coordinator = setup_coordinator().await;
        let order = create_order();
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8095);

        coordinator.send_ready_message(&order, &addr).await;

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_handler_screen_message_order_request() {
        let coordinator = setup_coordinator().await.start();
        let screen_addr = "127.0.0.1:0".parse().unwrap();
        let order = create_order();

        coordinator.send(ScreenMessage::OrderRequest { order: order.clone(), screen_addr }).await.unwrap();

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_handler_screen_message_commit_received() {
        let coordinator = setup_coordinator().await.start();
        let order = create_order();

        coordinator.send(ScreenMessage::CommitReceived { order: order.clone() }).await.unwrap();

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_handler_screen_message_abort() {
        let coordinator = setup_coordinator().await.start();
        let order = create_order();

        coordinator.send(ScreenMessage::Abort { order: order.clone() }).await.unwrap();

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_handler_robot_response_access_request() {
        let coordinator = setup_coordinator().await.start();
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let robot_id = 1;
        let flavors = vec![IceCreamFlavor::Vanilla];
        let flavors = flavors.into_iter().map(|flavor| (flavor, 1)).collect();

        coordinator.send(RobotResponse::AccessRequest { robot_id, flavors, addr }).await.unwrap();

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_handler_robot_response_release_request() {
        let coordinator = setup_coordinator().await.start();
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let robot_id = 1;
        let flavor = IceCreamFlavor::Vanilla;

        coordinator.send(RobotResponse::ReleaseRequest { robot_id, flavor, addr }).await.unwrap();

        // No assertion, just check that no panic occurs
    }

    #[actix_rt::test]
    async fn test_handler_robot_response_finish_request() {
        let coordinator = setup_coordinator().await.start();
        let robot_id = 2;
        let order = create_order();

        coordinator.send(RobotResponse::OrderFinished { robot_id, order }).await.unwrap();

        // No assertion, just check that no panic occurs
    }
}
