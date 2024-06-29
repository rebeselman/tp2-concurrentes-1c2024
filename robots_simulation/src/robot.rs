//! Represents a robot that can process orders
//! Each robot should be run in a separate process
use std::collections::HashSet;
use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use orders::{ice_cream_flavor::IceCreamFlavor, order::Order};

use actix::prelude::*;

use crate::{coordinator_messages::CoordinatorMessage, robot_messages::RobotResponse, robot_state::RobotState};



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
}

impl Robot {
    /// Creates a new robot
    /// # Arguments
    /// * `robot_id` - A usize representing the id of the robot
    /// * `socket` - An Arc<UdpSocket> representing the socket used to communicate with the coordinator
    /// * `server_addr` - A String representing the address of the coordinator
    pub fn new(robot_id: usize, socket: Arc<UdpSocket>, coordinator_addr: String) -> Self {
        Robot {
            robot_id,
            socket,
            coordinator_addr,
            state: RobotState::Idle,
        }
    }


    /// Makes a request to the coordinator
    fn make_request(&self, request: &RobotResponse) -> io::Result<()> {
        let mut message: Vec<u8> = b"access\n".to_vec();
        let request_serialized = serde_json::to_vec(request)?;
        message.extend_from_slice(&request_serialized);
        self.socket.send_to(&message, &self.coordinator_addr)?;
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

    fn process_allowed_access(&mut self,  flavor: IceCreamFlavor) -> io::Result<()>{

        let (order, flavors) = match &self.state {
            RobotState::WaitingForAccess(order, flavors) => (order.clone(), flavors.clone()),
            _ => return Ok(())
        };
    
        if flavors.contains(&flavor) {
            println!("[Robot {}] Access allowed for flavor {:?}", self.robot_id, &flavor);
        }
    
        thread::sleep(Duration::from_nanos(order.time_to_prepare() as u64 ));
        self.release_access(flavor.clone())?;
    
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

    fn process_received_order(&mut self, _robot_id: usize, order: Order) -> io::Result<()>{
        match self.state {
            RobotState::ProcessingOrder(_) => {
                println!("[Robot {}] Already processing an order", self.robot_id);
            
            }
            _ => {
                self.process_order(&order)?;
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
}

/// Implement the Actor trait for Robot
impl Actor for Robot {
    type Context = Context<Self>;
}

impl Handler<CoordinatorMessage>  for Robot {
    type Result = ();
    fn handle(&mut self, msg: CoordinatorMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            CoordinatorMessage::AccessAllowed { flavor } => {
                self.process_allowed_access(flavor).unwrap_or_else(|e|eprintln!("[Robot {}] Error processing allowed access: {}", self.robot_id, e))
            }
            CoordinatorMessage::AccessDenied { reason } => {
                self.process_denied_access(reason).unwrap_or_else(|e|eprintln!("[Robot {}] Error processing denied access: {}", self.robot_id, e))
                
            }
            CoordinatorMessage::OrderReceived { robot_id, order } => {
                self.process_received_order(robot_id, order).unwrap_or_else(|e|eprintln!("[Robot {}] Error processing received order: {}", self.robot_id, e))
                
            }
            CoordinatorMessage::OrderAborted { robot_id, order } => {
                self.abort_order(robot_id, order).unwrap_or_else(|e|eprintln!("[Robot {}] Error processing aborted order: {}", self.robot_id, e))
            }
            CoordinatorMessage::ACK => {
                println!("[Robot {}] ACK received", self.robot_id);
            }
        }
    }
    
}

