//! Represents a robot that can process orders
//! Each robot should be run in a separate process
use orders::{ice_cream_flavor::IceCreamFlavor, order::Order};
use std::collections::HashMap;
use std::{io, thread};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::Instant;
use chrono::Local;
use std::sync::Arc;
use std::time::Duration;

use actix::prelude::*;

use crate::{
    coordinator_messages::CoordinatorMessage, robot_messages::RobotResponse,
    robot_state::RobotState,
    election_message::ElectionMessage,
    election_state::ElectionState,
    ping_message::{PeerStatus, PingMessage},
    screen_message::ScreenMessage,
    udp_message_stream::UdpMessageStream,
    coordinator::Coordinator
};

const NUMBER_ROBOTS: usize = 5;



#[derive(Clone)]
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
    last_request_time: Option<Instant>, // New field to track the last request time
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
            last_request_time: None,
        }
    }

    /// Makes a request to the coordinator
    fn make_request(&self, request: &RobotResponse) -> io::Result<()> {
        let mut message: Vec<u8> = b"access\n".to_vec();
        let request_serialized = serde_json::to_vec(request)?;
        message.extend_from_slice(&request_serialized);
        self.send_to_socket(message, self.coordinator_addr.clone());
        Ok(())
    }

    /// Processes an order
    fn process_order(&mut self, order: &Order) -> io::Result<()> {
        let flavors_needed: HashMap<IceCreamFlavor, u32> = order.amounts_for_all_flavors();
        println!("[{}] [Robot {}] Processing order: {}", Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id, order.id());
        self.request_access(order, &flavors_needed)?;

        Ok(())
    }

    /// Requests access to the coordinator for a set of flavors
    /// Change the state of the robot to WaitingForAccess for that order and flavors
    /// # Arguments
    /// * `order` - An Order representing the order that the robot is processing
    /// * `flavors` - A Vec<IceCreamFlavor> representing the flavors that the robot needs access to

    fn request_access(&mut self, order: &Order, flavors: &HashMap<IceCreamFlavor, u32>) -> io::Result<()> {
        println!(
            "[Robot {}] Requesting access for flavors: {:?}",
            self.robot_id, flavors
        );
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
        println!("[{}] [ROBOT {}]: Releasing access to flavor {:?}", Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id, flavor);
        let request = RobotResponse::ReleaseRequest {
            robot_id: self.robot_id,
            flavor,
            addr: self.socket.local_addr()?,
        };
        self.last_request_time = None;
        self.make_request(&request)?;
        Ok(())
    }

    fn send_ping(&mut self) {
        let mut message: Vec<u8> = b"ping\n".to_vec();
        let ping_message = PingMessage::Ping;
        match serde_json::to_vec(&ping_message) {
            Ok(ping_serialized) => message.extend_from_slice(&ping_serialized),
            Err(e) => {
                eprintln!("Failed to serialize ping message: {:?}", e);
                return;
            },
        }
        if self.is_coordinator {
            self.ping_all_peers(&mut message);
        } else {
            // Only ping coordinator
            self.send_to_socket(message, self.coordinator_addr.clone());
        }
    }

    fn ping_all_peers(&mut self, message:  &mut [u8]) {
        for (_peer_addr, status) in &mut self.peers {
            status.ping_attempts += 1;
        }

        for (peer_addr, _status) in &self.peers {
            let addr = peer_addr.clone();
            self.send_to_socket(message.to_vec(), addr);
        }
    }

    fn check_peers_status(&mut self) {
        let now = Instant::now();
        let mut peers_to_remove = Vec::new();

        for (peer_addr, status) in &self.peers {
            if let Some(last_pong) = status.last_pong {
                let duration_since_last_pong = now.duration_since(last_pong);
                if duration_since_last_pong > Duration::from_secs(10) || status.ping_attempts >= 10 {
                    println!("[ROBOT {}] Peer {} has failed. Reassigning order", self.robot_id, peer_addr);
                    peers_to_remove.push(peer_addr.clone());
                    let peer_id = self.get_peer_id(peer_addr).unwrap_or_else(|| {
                        eprintln!("Failed to get peer id");
                        0
                    });
                    let request = RobotResponse::ReassignOrder { robot_id: peer_id };
                    if let Some(coordinator) = self.coordinator.clone() {
                        coordinator.do_send(request);
                    } else {
                        // Handle the case where coordinator is None, e.g., log an error or take corrective action
                        eprintln!("Coordinator is not available.");
                    }
                }
            } else {
                println!("[{}] [ROBOT {}] Peer {} has never responded. Ping attempts: {}",
                         Local::now().format("%Y-%m-%d %H:%M:%S"),
                         self.robot_id, peer_addr, status.ping_attempts);
            }
        }
        for peer_addr in peers_to_remove {
            println!("[{}] [ROBOT {}] Removing peer {}", Local::now().format("%Y-%m-%d %H:%M:%S"),
                     self.robot_id, peer_addr);
            self.peers.remove(&peer_addr);
        }
    }

    fn check_coordinador_status(&mut self) {
        // Find coordinator in peers
        if let Some(status) = self.peers.get(&self.coordinator_addr) {
            if let Some(last_pong) = status.last_pong {
                let now = Instant::now();
                let duration_since_last_pong = now.duration_since(last_pong);
                if duration_since_last_pong > Duration::from_secs(5) || status.ping_attempts >= 20 {
                    println!("[{}] [ROBOT {}] Coordinator {} has failed. Initiating election",
                             Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id, self.coordinator_addr);
                    self.election_state = ElectionState::StartingElection;
                }
            }
        }
    }

    fn get_peer_id(&self, peer: &str) -> Option<usize> {
        peer.chars().last().and_then(|c| c.to_digit(10)).map(|d| d as usize)
    }

    fn initiate_election(&mut self) {
        println!("[Robot {}] Initiating election", self.robot_id);
        let mut message: Vec<u8> = b"election\n".to_vec();
        let election_message = ElectionMessage::Election { robot_id: self.robot_id };
        match serde_json::to_vec(&election_message) {
            Ok(msg_serialized) => message.extend_from_slice(&msg_serialized),
            Err(e) => {
                eprintln!("Failed to serialize election message: {:?}", e);
                return;
            },
        }
        for (peer, _) in self.peers.iter() {
            // Get the id of the peer from last number in peer:
            let i = self.get_peer_id(peer);
            if let Some(digit) = i {
                if digit > self.robot_id {
                    println!("[ROBOT {}] Sending election message to {:?}", self.robot_id, i);
                    self.send_to_socket(message.clone(), peer.clone());
                }
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
            self.coordinator = Some(Coordinator::new(self.socket.clone(), self.robot_id).start());

            let mut message: Vec<u8> = b"election\n".to_vec();
            let election_message = ElectionMessage::NewCoordinator { robot_id: self.robot_id };
            match serde_json::to_vec(&election_message) {
                Ok(msg_serialized) => message.extend_from_slice(&msg_serialized),
                Err(e) => {
                    eprintln!("Failed to serialize election message: {:?}", e);
                    return;
                },
            }

            for (i, (peer, _)) in self.peers.iter().enumerate() {
                if i != self.robot_id {
                    let msg = message.clone();
                    let peer = peer.clone();
                    self.send_to_socket(msg, peer);
                }
            }
            self.election_state = ElectionState::None;
            self.send_current_order_to_new_coordinator().expect("Error sending order to new coordinator");
        } else {
            println!("[Robot {}] Election failed. I am not the new coordinator", self.robot_id);
        }
    }

    fn process_allowed_access(&mut self, flavor: IceCreamFlavor) -> io::Result<()> {
        let (order, mut flavors) = match &self.state {
            RobotState::WaitingForAccess(order, flavors) => (order.clone(), flavors.clone()),
            _ => return Ok(()),
        };
        self.state = RobotState::UsingContainer(flavor);
        if flavors.contains_key(&flavor) {
            println!(
                "[{}] [Robot {}] Access allowed for flavor {:?}", Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id, &flavor
            );
        }

        thread::sleep(Duration::from_millis(order.time_to_prepare() as u64));
        self.release_access(flavor)?;

        flavors.remove(&flavor);
        let flavor_needed = flavors.clone();
        if !flavor_needed.is_empty() {
            self.request_access(&order, &flavor_needed)?;
        } else {
            println!("[{}] [Robot {}] Order completed", Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id);
            let request = RobotResponse::OrderFinished {
                robot_id: self.robot_id,
                order: order.clone(),
            };
            self.make_request(&request)?;
            self.state = RobotState::Idle;
        }
        Ok(())
    }

    fn process_denied_access(&mut self, reason: String) -> io::Result<()> {
        println!("[Robot {}] Access denied. Reason: {}", self.robot_id, reason);
        let (order_clone, flavors_clone) = if let RobotState::WaitingForAccess(ref order, ref flavors) = self.state {
            (Some(order.clone()), Some(flavors.clone()))
        } else {
            (None, None)
        };

        // Now, use the cloned order and flavors for the request_access call if they exist.
        if let (Some(order), Some(flavors)) = (order_clone, flavors_clone) {
            thread::sleep(Duration::from_secs(2));
            self.request_access(&order, &flavors).expect("Error requesting access"
            );
        }
        Ok(())
    }

    fn process_received_order(&mut self, order: Order, screen_addr: &SocketAddr) -> io::Result<()>{
        if let RobotState::ProcessingOrder(_) = self.state {
            println!("[{}] [Robot {}] Already processing an order", Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id);
            Ok(())
        } else {
            self.process_order(&order).expect("Error processing order");
            self.order_screen_addr = Some(*screen_addr);
            Ok(())
        }
    }

    /// Sends current order to new coordinator, so it keeps track
    /// of all robot orders
    fn send_current_order_to_new_coordinator(&mut self) -> io::Result<()> {
        let order = match &self.state {
            RobotState::WaitingForAccess(order, _flavors) => order.clone(),
            RobotState::ProcessingOrder(order) => order.clone(),
            _ => return self.send_idle_message(),
        };
        // Send the order to the new coordinator
        self.send_order_in_process_message(&order)?;
        Ok(())
    }

    fn send_idle_message(&mut self) -> io::Result<()> {
        let request = RobotResponse::NoOrderInProcess {
            robot_id: self.robot_id,
            addr: self.socket.local_addr()?,
        };
        self.make_request(&request)?;
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
                if self.is_coordinator {
                    let coordinator = self.coordinator.clone().ok_or(io::Error::new(io::ErrorKind::Other, "Coordinator not set"))?;
                    coordinator.do_send(request);
                } else {
                    self.make_request(&request)?;
                }
            }
            None => {
                println!("[{}] [Robot {}] Order screen address not set", Local::now().format("%Y-%m-%d %H:%M:%S"), self.robot_id);
            }
        }
        Ok(())
    }

    fn abort_order(&mut self, _robot_id: usize, order: Order) -> io::Result<()> {
        match self.state {
            RobotState::WaitingForAccess(ref _waiting_order, _) => {
                self.state = RobotState::Idle;
                println!("[ROBOT {}] Order aborted: {:?}", self.robot_id, order.id());
            }
            RobotState::ProcessingOrder(ref _processing_order) => {
                self.state = RobotState::Idle;
                println!("[ROBOT {}] Order aborted: {:?}", self.robot_id, order.id());

            }
            RobotState::UsingContainer(ref flavor) => {
                self.release_access(*flavor).expect("Error releasing access");
                self.state = RobotState::Idle;
                println!("[ROBOT {}] Order aborted: {:?}", self.robot_id, order.id());
            }
            _ => {}
        }
        Ok(())
    }

    fn continue_order(&mut self) -> io::Result<()> {
        // Step 1: Clone necessary data before the match block
        let state_clone = self.state.clone();

        // Step 2: Adjust match to work with cloned data
        match state_clone {
            RobotState::WaitingForAccess(ref waiting_order, ref flavors) => {
                // Use cloned data
                self.request_access(waiting_order, flavors).expect("Error requesting access");
            }
            RobotState::ProcessingOrder(ref processing_order) => {
                // Use cloned data
                self.process_order(processing_order).expect("Error processing order");
            }
            _ => {}
        };

        Ok(())
    }

    fn handle_as_coordinator(&mut self, message_type: &str, mut parts: std::str::Split<char>, addr: SocketAddr) {
        let coordinator = match self.coordinator.clone() {
            Some(coordinator) => coordinator,
            None => {
                eprintln!("Coordinator not found.");
                return;
            }
        };        let message_type = message_type.to_string().clone();
        let content = match parts.next() {
            Some(part) => part.to_string(),
            None => {
                eprintln!("No more parts available.");
                return;
            }
        };
        actix_rt::spawn(async move {
            match message_type.as_str() {
                "prepare" => {
                    match serde_json::from_str::<Order>(content.as_str()) {
                        Ok(order) => {
                            println!("[COORDINATOR] Received prepare message for order: {}", order.id());
                            let order_request = ScreenMessage::OrderRequest {
                                order,
                                screen_addr: addr,
                            };
                            if let Err(e) = coordinator.send(order_request).await {
                                eprintln!("Failed to send OrderRequest: {}", e);
                            }
                        },
                        Err(e) => eprintln!("Failed to deserialize Order: {}", e),
                    }
                }
                "commit" => {
                    match serde_json::from_str::<Order>(content.as_str()) {
                        Ok(order) => {
                            let commit_received = ScreenMessage::CommitReceived { order };
                            if let Err(e) = coordinator.send(commit_received).await {
                                eprintln!("Failed to send CommitReceived: {}", e);
                            }
                        },
                        Err(e) => eprintln!("Failed to deserialize Order: {}", e),
                    }
                }
                "abort" => {
                    match serde_json::from_str::<Order>(content.as_str()) {
                        Ok(order) => {
                            println!("[COORDINATOR] Received abort message for order: {}", order.id());
                            let abort = ScreenMessage::Abort { order };
                            if let Err(e) = coordinator.send(abort).await {
                                eprintln!("Failed to send Abort message: {}", e);
                            }
                        },
                        Err(e) => eprintln!("Failed to deserialize Order: {}", e),
                    }
                }
                "access" => {
                    match serde_json::from_str::<RobotResponse>(content.as_str()) {
                        Ok(msg) => {
                            if let Err(e) = coordinator.send(msg).await {
                                eprintln!("Failed to send RobotResponse: {}", e);
                            }
                        },
                        Err(e) => eprintln!("Failed to deserialize RobotResponse: {}", e),
                    }
                }
                _ => {}
            };
        });
    }

    fn send_to_socket(&self, msg: Vec<u8>, addr: String) {
        let socket = self.socket.clone();
        actix_rt::spawn(async move {
            match socket.send_to(&msg, addr.clone()).await {
                Ok(_) => (),
                Err(e) => eprintln!("Failed to send message to {}: {}", addr, e),
            }
        });
    }

    fn handle_election_message(&mut self, message: ElectionMessage) {
        match message {
            ElectionMessage::Election { robot_id } => {
                if robot_id < self.robot_id {
                    // Send OK message
                    let mut message: Vec<u8> = b"election\n".to_vec();
                    let election_message = ElectionMessage::Ok { robot_id: self.robot_id };
                    match serde_json::to_vec(&election_message) {
                        Ok(election_serialized) => message.extend_from_slice(&election_serialized),
                        Err(e) => eprintln!("Failed to serialize election message: {:?}", e),
                    }
                    let addr = format!("127.0.0.1:809{}", robot_id);
                    self.send_to_socket(message, addr);
                    println!("[ROBOT {}] Received election message from {}", self.robot_id, robot_id);
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
                self.continue_order().expect("Error continuing order");
            }
            ElectionMessage::Ok { robot_id: _robot_id } => {
                println!("[ROBOT {}] Received OK message from {}. No longer a candidate", self.robot_id, _robot_id);
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
                    match serde_json::to_vec(&pong_message) {
                        Ok(pong_serialized) => {
                            message.extend_from_slice(&pong_serialized);
                            match cloned_socket.send_to(&message, addr).await {
                                Ok(_) => (),
                                Err(e) => eprintln!("Failed to send pong message: {}", e),
                            }
                        },
                        Err(e) => eprintln!("Failed to serialize pong message: {}", e),
                    }
                });
            }
            PingMessage::Pong => {
                // Update the status of the peer to 0 attempts
                self.update_last_pong(&addr);
            }
        }
    }

    fn update_last_pong(&mut self, addr: &SocketAddr) {
        if let Some(status) = self.peers.get_mut(&addr.to_string()) {
            status.last_pong = Some(Instant::now());
            status.ping_attempts = 0;
        }
    }

    fn handle_as_robot(&mut self, message: CoordinatorMessage) {
        match message {
            CoordinatorMessage::AccessAllowed { flavor } => {
                self.process_allowed_access(flavor).unwrap_or_else(|e| {
                    eprintln!(
                        "[Robot {}] Error processing allowed access: {}",
                        self.robot_id, e
                    )
                })
            }
            CoordinatorMessage::AccessDenied { reason } => {
                self.process_denied_access(reason).unwrap_or_else(|e| {
                    eprintln!(
                        "[Robot {}] Error processing denied access: {}",
                        self.robot_id, e
                    )
                })
            }
            CoordinatorMessage::OrderReceived { robot_id: _, order, screen_addr } => self
                .process_received_order(order, &screen_addr)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "[Robot {}] Error processing received order: {}",
                        self.robot_id, e
                    )
                }),
            CoordinatorMessage::OrderAborted { robot_id, order } => {
                self.abort_order(robot_id, order).unwrap_or_else(|e| {
                    eprintln!(
                        "[Robot {}] Error processing aborted order: {}",
                        self.robot_id, e
                    )
                })
            }
            CoordinatorMessage::ACK => {
                println!("[Robot {}] ACK received", self.robot_id);
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
        ctx.run_interval(Duration::from_secs(1), |robot, _ctx| {
            if robot.election_state == ElectionState::None {
                robot.send_ping();
            }
        });

        // Start checking peers' status at regular intervals
        ctx.run_interval(Duration::from_secs(2), |robot, _ctx| {
            if robot.election_state == ElectionState::None {
                if robot.is_coordinator {
                    robot.check_peers_status();
                } else {
                    robot.check_coordinador_status();
                }
            }
        });

        ctx.run_interval(Duration::from_secs(2), |robot, ctx| {
            if robot.election_state == ElectionState::StartingElection {
                robot.initiate_election();
                // Schedule the election result check after 5 seconds
                ctx.run_later(Duration::from_secs(5), |robot, _ctx| {
                    robot.check_election_results();
                });
            }
        });

        // Check for pending access requests and retry if necessary
        ctx.run_interval(Duration::from_secs(10), |robot, _ctx| {
            if robot.is_coordinator {
                return;
            }
            if let Some(last_request_time) = robot.last_request_time {
                if last_request_time.elapsed() > Duration::from_secs(5) { // Adjust the duration as needed
                    if let RobotState::WaitingForAccess(ref order, ref flavors) = robot.state {
                        println!("[{}] [Robot {}] Retrying access request for flavors: {:?}", Local::now().format("%Y-%m-%d %H:%M:%S"), robot.robot_id, flavors);
                        let order = order.clone();
                        let flavors = flavors.clone();
                        robot.request_access(&order, &flavors).unwrap_or_else(|e| {
                            eprintln!("[Robot {}] Error retrying access request: {}", robot.robot_id, e);
                        });
                    }
                }
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
            let message_type = parts.next().unwrap_or_else(|| {
                eprintln!("[Robot {}] Error receiving message", self.robot_id);
                ""
            });
            if message_type == "ping" {
                match parts.next() {
                    Some(part) => match serde_json::from_str::<PingMessage>(part) {
                        Ok(message) => self.handle_ping_message(message, addr),
                        Err(e) => eprintln!("[Robot {}] Failed to deserialize PingMessage: {}", self.robot_id, e),
                    },
                    None => eprintln!("[Robot {}] No message part available to deserialize", self.robot_id),
                }
            } else if message_type == "election" {
                match parts.next() {
                    Some(part) => match serde_json::from_str::<ElectionMessage>(part) {
                        Ok(message) => self.handle_election_message(message),
                        Err(e) => eprintln!("[Robot {}] Failed to deserialize ElectionMessage: {}", self.robot_id, e),
                    },
                    None => eprintln!("[Robot {}] No message part available to deserialize", self.robot_id),
                }
            } else if self.is_coordinator {
                self.update_last_pong(&addr);
                self.handle_as_coordinator(message_type, parts, addr);
            } else {
                match parts.next() {
                    Some(part) => match serde_json::from_str::<CoordinatorMessage>(part) {
                        Ok(message) => {
                            self.handle_as_robot(message);
                            self.update_last_pong(&addr);
                        },
                        Err(e) => eprintln!("[Robot {}] Failed to deserialize ElectionMessage: {}", self.robot_id, e),
                    },
                    None => eprintln!("[Robot {}] No message part available to deserialize", self.robot_id),
                }
            }
        } else {
            eprintln!("[Robot {}] Error receiving message", self.robot_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use orders::generate_orders;
    use crate::coordinator_messages::CoordinatorMessage;
    use super::*;

    #[actix_rt::test]
    async fn test_new_robot() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);
        let robot_peers = (0..NUMBER_ROBOTS).filter(|&id| id != robot_id)
            .map(|id| (format!("127.0.0.1:809{}", id), PeerStatus { last_pong: None, ping_attempts: 0 }))
            .collect();

        assert_eq!(robot.robot_id, robot_id);
        assert_eq!(robot.coordinator_addr, coordinator_addr);
        assert_eq!(robot.is_coordinator, is_coordinator);
        assert_eq!(robot.coordinator_id, Some(coordinator_id));
        assert_eq!(robot.state, RobotState::Idle);
        assert_eq!(robot.order_screen_addr, None);
        assert_eq!(robot.coordinator, None);
        assert_eq!(robot.peers, robot_peers);
        assert_eq!(robot.election_state, ElectionState::None);
        assert_eq!(robot.last_request_time, None);
    }

    #[actix_rt::test]
    async fn test_make_request() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;
        let mut flavors = HashMap::new();
        flavors.insert(IceCreamFlavor::Vanilla, 20);
        flavors.insert(IceCreamFlavor::Chocolate, 10);
        let addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

        let robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let request = RobotResponse::AccessRequest { robot_id, flavors, addr };
        let result = robot.make_request(&request);

        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn test_process_order() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        let result = robot.process_order(&order);

        assert!(result.is_ok());
        assert_eq!(robot.state, RobotState::WaitingForAccess(order.clone(), order.amounts_for_all_flavors()));
    }

    #[actix_rt::test]
    async fn test_request_access() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        let flavors = order.amounts_for_all_flavors();
        let result = robot.request_access(&order, &flavors);

        assert!(result.is_ok());
        assert_eq!(robot.state, RobotState::WaitingForAccess(order.clone(), flavors.clone()));
    }

    #[actix_rt::test]
    async fn test_release_access() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let flavor = IceCreamFlavor::Vanilla;
        let result = robot.release_access(flavor);

        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn test_send_ping() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        robot.send_ping();
        // Assert that the ping message is sent to all peers
    }

    #[actix_rt::test]
    async fn test_ping_all_peers() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut message: Vec<u8> = b"ping\n".to_vec();
        let ping_message = PingMessage::Ping;
        let ping_serialized = serde_json::to_vec(&ping_message).unwrap();
        message.extend_from_slice(&ping_serialized);

        robot.peers.insert(coordinator_addr.clone(), PeerStatus{ last_pong: Some(Instant::now()), ping_attempts: 0 });
        robot.ping_all_peers(&mut message);
        // Assert that the ping message is sent to all peers
    }

    #[actix_rt::test]
    async fn test_check_peers_status() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        robot.peers.insert(coordinator_addr.clone(), PeerStatus{ last_pong: Some(Instant::now()), ping_attempts: 0 });
        robot.check_peers_status();
        // Assert that the status of peers is checked and updated accordingly
    }

    #[actix_rt::test]
    async fn test_check_coordinator_status() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        robot.peers.insert(coordinator_addr.clone(), PeerStatus{ last_pong: Some(Instant::now()), ping_attempts: 0 });
        robot.check_coordinador_status();
        // Assert that the coordinator status is checked and updated accordingly
    }

    #[actix_rt::test]
    async fn test_initiate_election() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        robot.peers.insert(coordinator_addr.clone(), PeerStatus{ last_pong: Some(Instant::now()), ping_attempts: 0 });
        robot.initiate_election();
        // Assert that the election is initiated and messages are sent to all peers
    }

    #[actix_rt::test]
    async fn test_check_election_results() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        robot.election_state = ElectionState::Candidate;
        robot.check_election_results();
        // Assert that the election results are checked and appropriate actions are taken
    }

    #[actix_rt::test]
    async fn test_process_allowed_access() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let flavor = IceCreamFlavor::Vanilla;
        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        let flavors = order.amounts_for_all_flavors();
        robot.state = RobotState::WaitingForAccess(order, flavors);
        let result = robot.process_allowed_access(flavor);

        assert!(result.is_ok());
        // Assert that the robot processes the allowed access and updates its state accordingly
    }

    #[actix_rt::test]
    async fn test_process_denied_access() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let reason = "Flavor not available".to_string();
        let result = robot.process_denied_access(reason);

        assert!(result.is_ok());
        // Assert that the robot processes the denied access and updates its state accordingly
    }

    #[actix_rt::test]
    async fn test_process_received_order() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        let screen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000);
        let result = robot.process_received_order(order.clone(), &screen_addr);

        assert!(result.is_ok());
        // Assert that the robot processes the received order and updates its state accordingly
    }

    #[actix_rt::test]
    async fn test_send_current_order_to_new_coordinator() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        robot.state = RobotState::ProcessingOrder(order);
        let result = robot.send_current_order_to_new_coordinator();

        assert!(result.is_ok());
        // Assert that the current order is sent to the new coordinator
    }

    #[actix_rt::test]
    async fn test_send_idle_message() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let result = robot.send_idle_message();

        assert!(result.is_ok());
        // Assert that the idle message is sent to the coordinator
    }

    #[actix_rt::test]
    async fn test_send_order_in_process_message() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        robot.order_screen_addr = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000));
        let result = robot.send_order_in_process_message(&order);

        assert!(result.is_ok());
        // Assert that the order in process message is sent to the screen
    }

    #[actix_rt::test]
    async fn test_abort_order() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        let result = robot.abort_order(robot_id, order);

        assert!(result.is_ok());
        // Assert that the order is aborted and appropriate actions are taken
    }

    #[actix_rt::test]
    async fn test_continue_order() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);

        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        robot.state = RobotState::ProcessingOrder(order);
        let result = robot.continue_order();

        assert!(result.is_ok());
        // Assert that the order is continued and appropriate actions are taken
    }

    #[actix_rt::test]
    async fn test_handle_as_coordinator() {
        let robot_id = 2;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = true;

        let coordinator = Coordinator::new(socket.clone(), robot_id).start();
        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, robot_id);
        let message_type = "ping";
        let parts = "content".split(' ');
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000);

        robot.coordinator = Some(coordinator);
        robot.handle_as_coordinator(message_type, parts, addr);
        // Assert that the message is handled as a coordinator and appropriate actions are taken
    }

    #[actix_rt::test]
    async fn test_handle_election_message() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);
        let message = ElectionMessage::Election { robot_id: 2 };

        robot.handle_election_message(message);
        // Assert that the election message is handled and appropriate actions are taken
    }

    #[actix_rt::test]
    async fn test_handle_ping_message() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);
        let message = PingMessage::Ping;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000);

        robot.handle_ping_message(message, addr);
        // Assert that the ping message is handled and appropriate actions are taken
    }

    #[actix_rt::test]
    async fn test_update_last_pong() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9000);

        robot.peers.insert(addr.to_string(), PeerStatus{last_pong: None, ping_attempts: 0});
        robot.update_last_pong(&addr);
        // Assert that the last pong time is updated for the peer
    }

    #[actix_rt::test]
    async fn test_handle_as_robot() {
        let robot_id = 1;
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let coordinator_addr = "127.0.0.1:8080".to_string();
        let is_coordinator = false;
        let coordinator_id = 2;
        let screen_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);

        let mut robot = Robot::new(robot_id, socket, coordinator_addr.clone(), is_coordinator, coordinator_id);
        let mut rng = rand::thread_rng();
        let order = generate_orders::create_order_with_id(&mut rng, 1).unwrap();
        let message = CoordinatorMessage::OrderReceived {robot_id, order, screen_addr};

        robot.handle_as_robot(message);
        // Assert that the message is handled as a robot and appropriate actions are taken
    }
}