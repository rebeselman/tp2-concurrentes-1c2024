use std::collections::HashSet;
use std::hash::Hash;
use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use actix::prelude::*;
use super::items::{AccessAllowed, AccessDenied, IceCreamFlavor, Order, OrderReceived, RequestToCoordinator, RobotState};

pub struct Robot {
    robot_id: usize,
    socket: Arc<UdpSocket>,
    server_addr: String,
    state: RobotState,
}

impl Robot {
    pub fn new(robot_id: usize, socket: Arc<UdpSocket>, server_addr: String) -> Self {
        Robot {
            robot_id,
            socket,
            server_addr,
            state: RobotState::Idle,
        }
    }

    fn make_request(&self, request: &RequestToCoordinator) -> io::Result<()> {
        let mut message: Vec<u8> = b"access\n".to_vec();
        let request_serialized = serde_json::to_vec(request)?;
        message.extend_from_slice(&request_serialized);
        self.socket.send_to(&message, &self.server_addr)?;
        Ok(())
    }

    fn process_order(&mut self, order: &Order) -> io::Result<()> {
        let flavors: HashSet<IceCreamFlavor> = order.items.iter().flat_map(|item| item.flavors.clone()).collect();
        let flavors_needed: Vec<IceCreamFlavor> = flavors.into_iter().collect();
        println!("[Robot {}] Processing order: {:?}", self.robot_id, order);
        self.request_access(order, &flavors_needed)?;

        Ok(())
    }

    fn request_access(&mut self, order: &Order, flavors: &Vec<IceCreamFlavor>) -> io::Result<()> {
        println!("[Robot {}] Requesting access for flavors: {:?}", self.robot_id, flavors);
        self.state = RobotState::WaitingForAccess(order.clone(), flavors.clone());

        let request = RequestToCoordinator::SolicitarAcceso {
            robot_id: self.robot_id,
            flavors: flavors.clone(),
        };
        self.make_request(&request)?;
        Ok(())
    }

    fn release_access(&mut self, flavor: IceCreamFlavor) -> io::Result<()> {
        let request = RequestToCoordinator::LiberarAcceso {
            robot_id: self.robot_id,
            flavor,
        };
        self.make_request(&request)?;
        Ok(())
    }
}

impl Actor for Robot {
    type Context = Context<Self>;
}

impl Handler<OrderReceived> for Robot {
    type Result = ();

    fn handle(&mut self, msg: OrderReceived, _: &mut Self::Context) {
        match self.state {
            RobotState::ProcessingOrder(_) => {
                println!("[Robot {}] Already processing an order", self.robot_id);
            }
            _ => {
                self.process_order(&msg.order).unwrap();
            }
        }
    }
}

impl Handler<AccessAllowed> for Robot {
    type Result = ();

    fn handle(&mut self, msg: AccessAllowed, _: &mut Self::Context) {
        let (order, flavors) = match &self.state {
            RobotState::WaitingForAccess(order, flavors) => (order.clone(), flavors.clone()),
            _ => return,
        };

        if flavors.contains(&msg.flavor) {
            println!("[Robot {}] Access allowed for flavor {:?}", self.robot_id, msg.flavor);
            thread::sleep(Duration::from_secs(2));
            self.release_access(msg.flavor.clone()).unwrap();

            let flavor_needed: Vec<IceCreamFlavor> = flavors.into_iter().filter(|flavor| *flavor != msg.flavor).collect();

            if !flavor_needed.is_empty() {
                self.request_access(&order, &flavor_needed).unwrap();
            } else {
                println!("[Robot {}] Order completed", self.robot_id);
                let request = RequestToCoordinator::OrdenTerminada {
                    robot_id: self.robot_id,
                    order: order.clone(),
                };
                self.make_request(&request).unwrap();
                self.state = RobotState::Idle;
            }
        }
    }
}

impl Handler<AccessDenied> for Robot {
    type Result = ();

    fn handle(&mut self, msg: AccessDenied, _: &mut Self::Context) {
        if let RobotState::WaitingForAccess(ref order, ref flavors) = &self.state {
            println!("[Robot {}] Access denied. Reason: {}", self.robot_id, msg.reason);
        }
    }
}
