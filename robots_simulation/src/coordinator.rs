//! Coordinator module
//! This module contains the implementation of the Coordinator actor, which is responsible for managing the access to the ice cream containers and assigning orders to the robots.


use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use actix::{Actor, Context, Handler};
use super::order_status::OrderStatus;
use super::order_status_screen::OrderState;
use super::robot_messages::RobotResponse;
use super::screen_message::ScreenMessage;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use orders::ice_cream_flavor::IceCreamFlavor;
use orders::order::Order;
use serde_json::from_str;
use super::coordinator_messages::CoordinatorMessage::{self, AccessAllowed, AccessDenied, OrderReceived};

use super::order_status::OrderStatus::{Completed, CompletedButNotCommited, Pending};
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
    containers: HashMap<IceCreamFlavor, Arc<Mutex<bool>>>,
    socket: Arc<UdpSocket>,
    flavor_requests: Arc<Mutex<VecDeque<(Vec<IceCreamFlavor>, usize, SocketAddr)>>>,
    order_queue: Arc<Mutex<VecDeque<Order>>>,
    robot_states: Arc<Mutex<HashMap<usize, bool>>>, // Added to track robot states
    orders: HashMap<usize, OrderState>
}

const NUMBER_ROBOTS: usize = 5;

impl Coordinator {
    /// Creates a new Coordinator actor
    /// # Arguments
    /// * `socket` - An Arc<UdpSocket> representing the UDP socket used to communicate with the robots and the screen.
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        let flavors = vec![
            IceCreamFlavor::Chocolate,
            IceCreamFlavor::Strawberry,
            IceCreamFlavor::Vanilla,
            IceCreamFlavor::Mint,
            IceCreamFlavor::Lemon,
        ];
        // Faltaría modelar el stock de los contenedores!!!
        let containers = flavors.into_iter()
            .map(|flavor| (flavor, Arc::new(Mutex::new(false))))
            .collect();

        let robot_states = (1..NUMBER_ROBOTS).map(|id| (id, false)).collect(); // Initialize robot states

        Coordinator {
            containers,
            socket,
            flavor_requests: Arc::new(Mutex::new(VecDeque::new())),
            order_queue: Arc::new(Default::default()),
            robot_states: Arc::new(Mutex::new(robot_states)), // Add robot_states
            orders: HashMap::new()
        }
    }

    /// Assigns an order to a robot

    async fn assign_order_to_robot(&mut self, order: Order) {
        let mut robot_states = self.robot_states.lock().await;
        if let Some((&robot_id, &_busy)) = robot_states.iter().find(|&(_, &busy)| busy == false) {
            robot_states.insert(robot_id, true); // Mark robot as busy
            //let msg = serde_json::to_vec(&Response::AssignOrder { robot_id, order }).unwrap();

            // add robot id to order state
            if let Some(order_state) = self.orders.get_mut(&order.id()) {
                order_state.robot_id = Some(robot_id);
            }

            let robot_port: u16 = from_str::<u16>(format!("809{}", robot_id).as_str()).expect("Error parsing port");
            let addr: SocketAddr = SocketAddr::from(([0,0,0,0], robot_port));
            send_response(&self.socket, &OrderReceived { robot_id, order }, addr).await;
            println!("[COORDINATOR] Order assigned to robot {}", robot_id);
        } else {
            println!("All robots are busy");
            // All robots are busy, handle accordingly (e.g., add to a queue)
            self.order_queue.lock().await.push_back(order);
        }
    }

    /// Marks an order as completed
    /// # Arguments
    /// * `order_id` - A usize representing the ID of the order to mark as completed
    fn order_completed(&mut self, order_id: usize) {
        if let Some(order_state) = self.orders.get_mut(&order_id) {
            if order_state.status == OrderStatus::CommitReceived {
                order_state.status = OrderStatus::Completed;
            }
            else if order_state.status == OrderStatus::Pending {
                order_state.status = OrderStatus::CompletedButNotCommited;
            }
        }
    }
    /// Processes the queue of access requests to containers
    async fn process_queue(&self) {
        println!("[COORDINATOR] Checking if there are access requests in the queue");
        let mut flavor_requests_queue = self.flavor_requests.lock().await;
        while let Some((flavors, robot_id, robot_addr)) = flavor_requests_queue.pop_front() {
            self.check_if_flavor_available(robot_id, &flavors, robot_addr).await;
        }
    }

    /// Checks if a flavor is available and sends a response to the robot
    async fn check_if_flavor_available(&self, robot_id: usize, flavors: &Vec<IceCreamFlavor>, addr: SocketAddr) {
        for flavor in flavors {
            let container = self.containers.get(flavor).unwrap().clone();
            let mut access = container.lock().await;
            let response = if !*access {
                *access = true;
                println!("[COORDINATOR] Robot {} is requesting access to container {:?}", robot_id, flavor);
                //Response::AccesoConcedido(flavor.clone())

                AccessAllowed { flavor: flavor.clone() }
            } else {
                //Response::AccesoDenegado("Container already in use".into())
                AccessDenied { reason: "Container already in use".into() }
            };
            send_response(&self.socket, &response, addr).await;
            if matches!(response, AccessAllowed { .. }) {
                return;
            }
        }
        //let response = Response::AccesoDenegado("All requested containers are in use".into());
        let response = AccessDenied { reason: "All requested containers are in use".into() };
        self.flavor_requests.lock().await.push_back((flavors.clone(), robot_id, addr));
        send_response(&self.socket, &response, addr).await;
    }


    /// Sends a finish message to the screen
    fn send_finish_message(&self, order_id: usize, addr: &SocketAddr) {
        let socket = self.socket.clone();
        let addr = addr.clone();
        actix_rt::spawn(async move {
            let message = format!("{}\nfinished", order_id).into_bytes();
            socket.send_to(&message, &addr).await.unwrap();
        });
    }


    /// Send abort message to the screen
    fn send_abort_message(&self, order_id: usize, addr: &SocketAddr) {
        let socket = self.socket.clone();
        let addr = addr.clone();
        actix_rt::spawn(async move {
            let message = format!("{}\nabort", order_id).into_bytes();
            socket.send_to(&message, &addr).await.unwrap();
        });
    }

    fn commit_received(&mut self, order: &Order) {
        let mut send_finished = false;
        let mut addr: SocketAddr = SocketAddr::new([0, 0, 0, 0].into(), 0);
        if let Some(order_state) = self.orders.get_mut(&order.id()) {
            if order_state.status == Pending {
                order_state.status = OrderStatus::CommitReceived;
            } else if order_state.status == CompletedButNotCommited {
                order_state.status = Completed;
                send_finished = true;
                addr = order_state.screen_addr.clone();
            }
        }
        // If address is not null
        if send_finished && addr != SocketAddr::new([0, 0, 0, 0].into(), 0) {
            self.send_finish_message(order.id(), &addr);
        }
    }

    fn abort_order(&mut self, order: Order) {
        println!("Order aborted: {}", order.id());
        // remove the order from the orders
        if let Some(order_state) = self.orders.remove(&order.id()) {
            // stop the robot?
            let this = self.clone();
            actix_rt::spawn(async move {
                // if some robot was assigned to the order
                if let Some(robot_id) = order_state.robot_id {
                    // send abort message to the robot
                    let msg = serde_json::to_vec(&CoordinatorMessage::OrderAborted { robot_id, order }).unwrap();
                    this.socket.clone().send_to(&msg, format!("127.0.0.1:809{}", robot_id)).await.unwrap();
                    // change my states as coordinator
                    let mut robot_states = this.robot_states.lock().await;
                    robot_states.insert(robot_id, false);
                } else {
                    // if no robot was assigned to the order
                    // remove the order from the order queue
                    let mut order_queue = this.order_queue.lock().await;
                    order_queue.retain(|o| o.id() != order.id());
                }
                // send abort message to the screen
                let addr: SocketAddr = SocketAddr::new(order_state.screen_addr.ip(), order_state.screen_addr.port());
                this.send_abort_message(order_state.order.id(), &addr);
            });
        }
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
                let order = order.clone();
                let addr = screen_addr.clone();
                let mut this = self.clone();
                self.orders.insert(order.id(), OrderState {
                    order: order.clone(),
                    status: Pending,
                    screen_addr,
                    robot_id: None,

                });
                actix_rt::spawn(async move {
                    let message = format!("{}\nready", order.id()).into_bytes();
                    this.socket.send_to(&message, &addr).await.unwrap();
                    this.assign_order_to_robot(order).await;
                });
            }
            ScreenMessage::CommitReceived { order } => {
                self.commit_received(&order);
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
            RobotResponse::AccessRequest { robot_id, flavors, addr } => {
                let flavors = flavors.clone();
                let this = self.clone();
                actix_rt::spawn(async move {
                    this.check_if_flavor_available(robot_id, &flavors, addr).await;
                });
            }
            RobotResponse::ReleaseRequest { robot_id, flavor, addr } => {
                let container = self.containers.get(&flavor).unwrap().clone();
                let socket = self.socket.clone();
                let this = self.clone();
                println!("[COORDINATOR] Releasing access for container {:?} from {:?}", flavor, robot_id);
                actix_rt::spawn(async move {
                    *container.lock().await = false;
                    send_response(&socket, &CoordinatorMessage::ACK, addr).await;
                });
                actix_rt::spawn(async move {
                    this.process_queue().await;
                });

            }
            RobotResponse::OrderFinished { robot_id, order } => {
                let order_id = order.id();
                self.order_completed(order_id);
                if let Some(order_state) = self.orders.get(&order_id) {
                    if order_state.status == Completed {
                        self.send_finish_message(order_id, &order_state.screen_addr);
                    }
                }
                let robot_states = self.robot_states.clone();
                actix_rt::spawn(async move {
                    let mut robot_states = robot_states.lock().await;
                    robot_states.insert(robot_id, false);
                });
            }
        }
    }
}
