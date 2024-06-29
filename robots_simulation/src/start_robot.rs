use actix::{Actor, System};
use std::net::UdpSocket;
use std::sync::Arc;
use std::io;
use std::time::Duration;
use robots_simulation::robot::Robot;
use robots_simulation::coordinator_messages::CoordinatorMessage::{self, AccessAllowed, AccessDenied, OrderReceived, ACK};
fn main() -> io::Result<()> {
    let robot_id: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let addr = format!("127.0.0.1:809{}", robot_id);
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
                let message: Result<CoordinatorMessage, _> = serde_json::from_slice(&buf[..amt]);
                match message {
            
                    Ok(OrderReceived { robot_id, order }) => {
                        robot.send(OrderReceived { robot_id, order }).await.unwrap();
                    }
                    Ok(AccessAllowed { flavor }) => {
                        robot.send(AccessAllowed { flavor }).await.unwrap();
                    }
                    Ok(AccessDenied { reason }) => {
                        robot.send(AccessDenied { reason }).await.unwrap();
                    }
                    Ok(CoordinatorMessage::OrderAborted { robot_id, order }) => {
                        robot.send(CoordinatorMessage::OrderAborted { robot_id, order }).await.unwrap();
                    }
                    Ok(ACK) => {
                        println!("ACK received");
                    }
                    _ => {}
                }
            // tokio::time::sleep(Duration::from_millis(100)).await;
            }
            
        }
    });
    

    system.run()
}
