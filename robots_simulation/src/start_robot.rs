use actix::{Actor, System};
use std::net::UdpSocket;
use std::sync::Arc;
use std::io;
use std::time::Duration;
use serde_json::Error;
use robots_simulation::robot::Robot;
use robots_simulation::items::{AccessAllowed, AccessDenied, OrderReceived, RequestToCoordinator, Response};

fn main() -> io::Result<()> {
    let robot_id: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let addr = format!("0.0.0.0:809{}", robot_id);
    let socket = UdpSocket::bind(&addr)?;
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    let server_addr = "127.0.0.1:8080".to_string();
    let socket = Arc::new(socket);

    let system = System::new();

    println!("Robot {} is ready", robot_id);
    system.block_on(async {
        let robot = Robot::new(robot_id, Arc::clone(&socket), server_addr).start();

        loop {
            let mut buf = [0; 1024];
            if let Ok((amt, _)) = socket.recv_from(&mut buf) {
                let message: Result<Response, _> = serde_json::from_slice(&buf[..amt]);
                match message {
                    Ok(Response::AssignOrder {robot_id, order }) => {
                        let order_msg = OrderReceived {
                            robot_id,
                            order,
                        };
                        robot.send(order_msg).await.unwrap();
                    }
                    Ok(Response::AccesoConcedido(flavor)) => {
                        let access_msg = AccessAllowed {
                            flavor,
                        };
                        robot.send(access_msg).await.unwrap();
                    }
                    Ok(Response::AccesoDenegado(reason)) => {
                        let deny_msg = AccessDenied {
                            reason,
                        };
                        robot.send(deny_msg).await.unwrap();
                    }
                    Ok(Response::ACK) => {
                        println!("ACK received");
                    }
                    _ => {}
                }
            }
            // tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    system.run()
}
