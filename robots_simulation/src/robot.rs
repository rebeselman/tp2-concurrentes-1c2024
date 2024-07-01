//! Represents a robot that can process orders
//! Each robot should be run in a separate process
use std::collections::{HashMap, HashSet};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use actix::prelude::*;
use orders::{ice_cream_flavor::IceCreamFlavor, order::Order};
use tokio::net::UdpSocket;
use tokio::time::Instant;

use crate::{coordinator_messages::CoordinatorMessage, robot_messages::RobotResponse, robot_state::RobotState};
use crate::election_message::ElectionMessage;
use crate::election_state::ElectionState;
use crate::ping_message::{PeerStatus, PingMessage};
use crate::screen_message::ScreenMessage;
use crate::udp_message_stream::UdpMessageStream;

use super::coordinator::Coordinator;

const NUMBER_ROBOTS: usize = 5;



/// Represents a robot that can process orders
/// Contains:
/// * robot_id: usize - The id of the robot
/// * socket: Arc<UdpSocket> - The socket used to communicate with the coordinator
/// * coordinator_addr: String - The address of the coordinator ?
/// * state: RobotState - The current state of the robot
pub struct Robot {
    robot_id: usize,
    socket: Arc<UdpSocket>,
    coordinator_addr: String,
    state: RobotState,
    order_screen_addr: Option<SocketAddr>,
    is_coordinator: bool,
    pub coordinator: Option<Addr<Coordinator>>,
    peers: HashMap<String, PeerStatus>,
    coordinator_id: Option<usize>,
    election_state: ElectionState,
}

impl Robot {
    /// Creates a new robot
    /// # Arguments
    /// * `robot_id` - A usize representing the id of the robot
    /// * `socket` - An Arc<UdpSocket> representing the socket used to communicate with the coordinator
    /// * `server_addr` - A String representing the address of the coordinator
    pub fn new(robot_id: usize, socket: Arc<UdpSocket>, coordinator_addr: String, is_coordinator: bool, coordinator_id: usize) -> Self {
        Robot {
            robot_id,
            socket,
            coordinator_addr,
            state: RobotState::Idle,
            order_screen_addr: None,
            is_coordinator,
            coordinator: None,
            peers: (0..NUMBER_ROBOTS).filter(|&id| id != robot_id).map(|id| (format!("127.0.0.1:809{}", id), PeerStatus { last_pong: None, ping_attempts: 0 })).collect(),
            coordinator_id: Some(coordinator_id),
            election_state: ElectionState::None,
        }
    }


    /// Makes a request to the coordinator
    fn make_request(&self, request: &RobotResponse) -> io::Result<()> {
        let mut message: Vec<u8> = b"access\n".to_vec();
        let request_serialized = serde_json::to_vec(request)?;
        message.extend_from_slice(&request_serialized);
        let socket = self.socket.clone();
        let coordinator_addr = self.coordinator_addr.clone();
        actix_rt::spawn(async move {
            socket.send_to(&message, &coordinator_addr).await.unwrap();
        });
        Ok(())
    }

    /// Processes an order
    fn process_order(&mut self, order: &Order) -> io::Result<()> {
        let flavors: HashSet<IceCreamFlavor> = order.items().iter().flat_map(|item| item.flavors().clone()).collect();
        let flavors_needed: Vec<IceCreamFlavor> = flavors.into_iter().collect();
        println!("[Robot {}] Processing order: {:?}", self.robot_id, order);
        self.request_access(order, &flavors_needed)?;

        Ok(())
    }



    /// Requests access to the coordinator for a set of flavors
    /// Change the state of the robot to WaitingForAccess for that order and flavors
    /// # Arguments
    /// * `order` - An Order representing the order that the robot is processing
    /// * `flavors` - A Vec<IceCreamFlavor> representing the flavors that the robot needs access to

    fn request_access(&mut self, order: &Order, flavors: &Vec<IceCreamFlavor>) -> io::Result<()> {
        println!("[Robot {}] Requesting access for flavors: {:?}", self.robot_id, flavors);
        self.state = RobotState::WaitingForAccess(order.clone(), flavors.clone());

        let request = RobotResponse::AccessRequest {
            robot_id: self.robot_id,
            flavors: flavors.clone(),
            addr: self.socket.local_addr()?,
        };

        self.make_request(&request)?;
        Ok(())
    }

    /// Releases access to a flavor
    /// # Arguments
    /// * `flavor` - An IceCreamFlavor representing the flavor that the robot is releasing access to
    fn release_access(&mut self, flavor: IceCreamFlavor) -> io::Result<()> {
        let request = RobotResponse::ReleaseRequest {
            robot_id: self.robot_id,
            flavor,
            addr: self.socket.local_addr()?,
        };
        self.make_request(&request)?;
        Ok(())
    }

    fn send_ping(&mut self) {
        let mut message: Vec<u8> = b"ping\n".to_vec();
        let ping_message = PingMessage::Ping;
        let ping_serialized = serde_json::to_vec(&ping_message).unwrap();
        message.extend_from_slice(&ping_serialized);
        if self.is_coordinator {
            self.ping_all_peers(&mut message);
        } else {
            // Only ping coordinator
            let coordinator_addr = self.coordinator_addr.clone();
            let socket = self.socket.clone();
            actix_rt::spawn(async move {
                socket.send_to(&message, coordinator_addr).await.unwrap();
            });
        }
    }

    fn ping_all_peers(&mut self, message:  &mut [u8]) {
        for (peer_addr, status) in &mut self.peers {
            let message_to_peer = message.to_owned();
            let socket = self.socket.clone();
            let addr = peer_addr.clone();
            actix_rt::spawn(async move {
                socket.send_to(&message_to_peer, addr).await.unwrap();
            });
            status.ping_attempts += 1;
        }
    }

    fn check_peers_status(&mut self) {
        let now = Instant::now();
        let mut peers_to_remove = Vec::new();

        for (peer_addr, status) in &self.peers {
            if let Some(last_pong) = status.last_pong {
                let duration_since_last_pong = now.duration_since(last_pong);
                /*println!("Robot {}: Peer {} last pong received {} seconds ago. Ping attempts: {}",
                         self.robot_id, peer_addr, duration_since_last_pong.as_secs(), status.ping_attempts);*/
                if duration_since_last_pong > Duration::from_secs(20) && status.ping_attempts >= 5 {
                    println!("Robot {}: Peer {} has failed. Reassigning order", self.robot_id, peer_addr);
                    peers_to_remove.push(peer_addr.clone());
                    let peer_id = peer_addr.chars().last().unwrap().to_digit(10).unwrap() as usize;
                    let request = RobotResponse::ReassignOrder { robot_id: peer_id };
                    let coordinator = self.coordinator.clone().unwrap();
                    coordinator.do_send(request);
                }
            } else {
                println!("Robot {}: Peer {} has never responded. Ping attempts: {}",
                         self.robot_id, peer_addr, status.ping_attempts);
            }
        }
        for peer_addr in peers_to_remove {
            self.peers.remove(&peer_addr);
        }
    }

    fn check_coordinador_status(&mut self) {
        // Find coordinator in peers
        if let Some(status) = self.peers.get(&self.coordinator_addr) {
            if let Some(last_pong) = status.last_pong {
                let now = Instant::now();
                let duration_since_last_pong = now.duration_since(last_pong);
                if duration_since_last_pong > Duration::from_secs(20) && status.ping_attempts >= 5 {
                    println!("Robot {}: Coordinator {} has failed. Initiating election", self.robot_id, self.coordinator_addr);
                    self.election_state = ElectionState::StartingElection;
                }
            }
        }
    }

    fn initiate_election(&mut self) {
        println!("[Robot {}] Initiating election", self.robot_id);
        let mut message: Vec<u8> = b"election\n".to_vec();
        let election_message = ElectionMessage::Election { robot_id: self.robot_id };
        let msg_serialized = serde_json::to_vec(&election_message).unwrap();
        message.extend_from_slice(&msg_serialized);
        for (peer, _) in self.peers.iter() {
            // Get the id of the peer from last number in peer:
            let i = peer.chars().last().unwrap().to_digit(10).unwrap() as usize;
            if i > self.robot_id {
                println!("Robot {}: Sending election message to {}", self.robot_id, i);
                let peer = peer.clone();
                let socket = self.socket.clone();
                let msg = message.clone();
                actix_rt::spawn(async move {
                    socket.send_to(&msg, peer).await.unwrap();
                });
            }
        }
        self.election_state = ElectionState::Candidate;
    }

    fn check_election_results(&mut self) {
        if self.election_state == ElectionState::Candidate {
            println!("[Robot {}] Election successful. I am the new coordinator", self.robot_id);
            self.election_state = ElectionState::None;
            self.is_coordinator = true;
            self.coordinator_id = Some(self.robot_id);
            self.coordinator = Some(Coordinator::new(self.socket.clone()).start());

            let mut message: Vec<u8> = b"election\n".to_vec();
            let election_message = ElectionMessage::NewCoordinator { robot_id: self.robot_id };
            let msg_serialized = serde_json::to_vec(&election_message).unwrap();
            message.extend_from_slice(&msg_serialized);

            for (i, (peer, _)) in self.peers.iter().enumerate() {
                if i != self.robot_id {
                    let socket = self.socket.clone();
                    let msg = message.clone();
                    let peer = peer.clone();
                    actix_rt::spawn(async move {
                        socket.send_to(&msg, peer).await.unwrap();
                    });
                }
            }
            self.election_state = ElectionState::None;
        } else {
            println!("[Robot {}] Election failed. I am not the new coordinator", self.robot_id);
        }
    }

    fn process_allowed_access(&mut self,  flavor: IceCreamFlavor) -> io::Result<()>{

        let (order, flavors) = match &self.state {
            RobotState::WaitingForAccess(order, flavors) => (order.clone(), flavors.clone()),
            _ => return Ok(())
        };

        if flavors.contains(&flavor) {
            println!("[Robot {}] Access allowed for flavor {:?}", self.robot_id, &flavor);
        }

        thread::sleep(Duration::from_secs(order.time_to_prepare() as u64 ));
        self.release_access(flavor)?;

        let flavor_needed: Vec<IceCreamFlavor> = flavors.into_iter().filter(|other| *other != flavor).collect();

        if !flavor_needed.is_empty() {
            self.request_access(&order, &flavor_needed)?;
        } else {
            println!("[Robot {}] Order completed", self.robot_id);

            let request = RobotResponse::OrderFinished {
                robot_id: self.robot_id,
                order: order.clone(),
            };
            self.make_request(&request)?;
            self.state = RobotState::Idle;
        }
        Ok(())

    }

    fn process_denied_access(&mut self, reason: String) -> io::Result<()>{
        if let RobotState::WaitingForAccess(ref _order, ref _flavors) = &self.state {
            println!("[Robot {}] Access denied. Reason: {}", self.robot_id, reason);
        }
        Ok(())
    }

    fn process_received_order(&mut self, _robot_id: usize, order: Order, screen_addr: &SocketAddr) -> io::Result<()>{
        match self.state {
            RobotState::ProcessingOrder(_) => {
                println!("[Robot {}] Already processing an order", self.robot_id);

            }
            _ => {
                self.process_order(&order)?;
                self.order_screen_addr = Some(*screen_addr);
            }
        }
        Ok(())
    }

    /// Sends current order to new coordinator, so it keeps track
    /// of all robot orders
    fn send_current_order_to_new_coordinator(&mut self) -> io::Result<()> {
        let order = match &self.state {
            RobotState::WaitingForAccess(order, _flavors) => order.clone(),
            RobotState::ProcessingOrder(order) => order.clone(),
            _ => return Ok(())
        };
        // Send the order to the new coordinator
        self.send_order_in_process_message(&order)?;
        Ok(())
    }

    fn send_order_in_process_message(&mut self, order: &Order) -> io::Result<()> {
        // If self.order_screen_addr is Some, send request
        match self.order_screen_addr {
            Some(screen_addr) => {
                let request = RobotResponse::OrderInProcess {
                    robot_id: self.robot_id,
                    order: order.clone(),
                    addr: self.socket.local_addr()?,
                    screen_addr
                };
                self.make_request(&request)?;
            }
            None => {
                println!("[Robot {}] Order screen address not set", self.robot_id);
            }
        }
        Ok(())
    }

    fn abort_order(&mut self, _robot_id: usize, order: Order) -> io::Result<()>{
        match self.state {

            RobotState::WaitingForAccess(ref _waiting_order, _) => {
                self.state = RobotState::Idle;
                println!("[ROBOT {} ]Order aborted: {:?}", self.robot_id, order.id());
            }
            RobotState::ProcessingOrder(ref _processing_order) => {
                self.state = RobotState::Idle;

                println!("[ROBOT {} ]Order aborted: {:?}", self.robot_id, order.id());

            }
            _ => {}
        }
        Ok(())
    }

    fn handle_as_coordinator(&mut self, message_type: &str, mut parts: std::str::Split<char>, addr: SocketAddr) {
        let coordinator = self.coordinator.clone().unwrap();
        let message_type = message_type.to_string().clone();
        let content = parts.next().unwrap().to_string().clone();
        actix_rt::spawn(async move {
            match message_type.as_str() {
                "prepare" => {
                    let order: Order = serde_json::from_str(content.as_str()).unwrap();
                    println!("[COORDINATOR] Received prepare message for order: {}", order.id());
                    let order_request = ScreenMessage::OrderRequest {
                        order,
                        screen_addr: addr,
                    };
                    coordinator.send(order_request).await.unwrap();
                }
                "commit" => {
                    let order: Order = serde_json::from_str(content.as_str()).unwrap();
                    println!("[COORDINATOR] Received commit message for order: {}", order.id());
                    let commit_received = ScreenMessage::CommitReceived {
                        order
                    };
                    coordinator.send(commit_received).await.unwrap();
                }
                "abort" => {
                    let order: Order = serde_json::from_str(content.as_str()).unwrap();
                    println!("[COORDINATOR] Received abort message for order: {}", order.id());

                    let abort = ScreenMessage::Abort {
                        order,
                    };
                    coordinator.send(abort).await.unwrap();
                }
                "access" => {
                    let msg: RobotResponse = serde_json::from_str(content.as_str()).unwrap();

                    coordinator.send(msg).await.unwrap();
                },
                _ => {}
            };
        });
    }

    fn handle_election_message(&mut self, message: ElectionMessage) {
        match message {
            ElectionMessage::Election { robot_id } => {
                if robot_id < self.robot_id {
                    // Send OK message
                    let mut message: Vec<u8> = b"election\n".to_vec();
                    let election_message = ElectionMessage::Ok { robot_id: self.robot_id };
                    let election_serialized = serde_json::to_vec(&election_message).unwrap();
                    message.extend_from_slice(&election_serialized);
                    let socket = self.socket.clone();
                    let addr = format!("127.0.0.1:809{}", robot_id);
                    actix_rt::spawn(async move {
                        socket.send_to(&message, addr).await.unwrap();
                    });
                    println!("Robot {}: Received election message from {}", self.robot_id, robot_id);
                    if self.election_state == ElectionState::None {
                        self.election_state = ElectionState::StartingElection;
                    }
                }
            }
            ElectionMessage::NewCoordinator { robot_id } => {
                self.is_coordinator = false;
                self.election_state = ElectionState::None;
                self.coordinator_addr = format!("127.0.0.1:809{}", robot_id);
                self.send_current_order_to_new_coordinator().expect("Error sending order to new coordinator");
            }
            ElectionMessage::Ok { robot_id: _robot_id } => {
                println!("Robot {}: Received OK message from {}. No longer a candidate", self.robot_id, _robot_id);
                self.election_state = ElectionState::Follower;
            }
        }
    }

    fn handle_ping_message(&mut self, message: PingMessage, addr: SocketAddr) {
        match message {
            PingMessage::Ping => {
                // Send a Pong response
                let cloned_socket = self.socket.clone();
                actix_rt::spawn(async move {
                    let mut message: Vec<u8> = b"ping\n".to_vec();
                    let pong_message = PingMessage::Pong;
                    let pong_serialized = serde_json::to_vec(&pong_message).unwrap();
                    message.extend_from_slice(&pong_serialized);
                    cloned_socket.send_to(&message, addr).await.unwrap();
                });
                // println!("Robot {}: Sent pong to {}", self.robot_id, addr);
            }
            PingMessage::Pong => {
                // println!("Robot {}: Received pong from {}", self.robot_id, addr);
                // Update the status of the peer to 0 attempts
                if let Some(status) = self.peers.get_mut(&addr.to_string()) {
                    // println!("Robot {}: Peer {} responded", self.robot_id, addr);
                    status.last_pong = Some(Instant::now());
                    status.ping_attempts = 0;
                }
            }
        }
    }


    fn handle_as_robot(&mut self, message: CoordinatorMessage) {
        match message {
            CoordinatorMessage::AccessAllowed { flavor } => {
                self.process_allowed_access(flavor).unwrap();
            }
            CoordinatorMessage::AccessDenied { reason } => {
                self.process_denied_access(reason).unwrap();
            }
            CoordinatorMessage::OrderReceived { robot_id, order, screen_addr } => {
                self.process_received_order(robot_id, order, &screen_addr).unwrap();
            }
            CoordinatorMessage::OrderAborted { robot_id, order } => {
                self.abort_order(robot_id, order).unwrap();
            }
            CoordinatorMessage::ACK => {
                // println!("[Robot {}] ACK received", self.robot_id);
            }
        }
    }
}

/// Implement the Actor trait for Robot
impl Actor for Robot {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        println!("Robot {} started", self.robot_id);

        // Start sending pings at regular intervals
        ctx.run_interval(Duration::from_secs(5), |robot, _ctx| {
            if robot.election_state == ElectionState::None {
                robot.send_ping();
            }
        });

        // Start checking peers' status at regular intervals
        ctx.run_interval(Duration::from_secs(10), |robot, _ctx| {
            if robot.election_state == ElectionState::None {
                if robot.is_coordinator {
                    robot.check_peers_status();
                } else {
                    robot.check_coordinador_status();
                }
            }
        });

        ctx.run_interval(Duration::from_secs(8), |robot, ctx| {
            if robot.election_state == ElectionState::StartingElection {
                robot.initiate_election();
                // Schedule the election result check after 10 seconds
                ctx.run_later(Duration::from_secs(10), |robot, _ctx| {
                    robot.check_election_results();
                });
            }
        });

        let stream = UdpMessageStream::new(self.socket.clone());
        ctx.add_stream(stream);
    }
}

impl StreamHandler<io::Result<(usize, Vec<u8>, SocketAddr)>> for Robot {
    fn handle(&mut self, item: io::Result<(usize, Vec<u8>, SocketAddr)>, _ctx: &mut Self::Context) {
        if let Ok((len, buf, addr)) = item {
            let received_message = String::from_utf8_lossy(&buf[..len]);
            let mut parts = received_message.split('\n');
            let message_type = parts.next().unwrap();
            // println!("Robot {}: Received message: {}", self.robot_id, message_type);
            if message_type == "ping" {
                let message: PingMessage = serde_json::from_str(parts.next().unwrap()).unwrap();
                self.handle_ping_message(message, addr);
            } else if message_type == "election" {
                let message: ElectionMessage = serde_json::from_str(parts.next().unwrap()).unwrap();
                self.handle_election_message(message);
            } else if self.is_coordinator {
                // println!("Handling as coordinator");
                self.handle_as_coordinator(message_type, parts, addr);
            } else {
                let message: CoordinatorMessage = serde_json::from_str(parts.next().unwrap()).unwrap();
                self.handle_as_robot(message);
            }
        } else {
            eprintln!("[Robot {}] Error receiving message", self.robot_id);
        }
    }
}
