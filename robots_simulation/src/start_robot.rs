use actix::{Actor, System};
use tokio::net::UdpSocket;
use std::sync::Arc;
use std::io;
use robots_simulation::coordinator::Coordinator;
use robots_simulation::robot::Robot;

fn build() -> io::Result<(usize, usize, String)> {
    let robot_id: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    let is_coordinator: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let addr = format!("127.0.0.1:809{}", robot_id);

    return Ok((robot_id, is_coordinator, addr));
}


fn main() -> io::Result<()> {
    let system = System::new();
    let (robot_id, coordinator_id, addr) = build()?;
    system.block_on(async {
        let socket: UdpSocket = UdpSocket::bind(&addr).await.unwrap();
        let socket = Arc::new(socket);
        let coordinator_addr = format!("127.0.0.1:809{}", coordinator_id);
        let is_coordinator = robot_id == coordinator_id;

        let mut robot = Robot::new(robot_id, socket.clone(), coordinator_addr.clone(), is_coordinator, coordinator_id);

        if is_coordinator {
            println!("Robot {} is the coordinator", robot_id);
            let coordinator = Coordinator::new(socket.clone());
            robot.coordinator = Some(coordinator.start());
        }
        robot.start();
    });

    system.run()
}