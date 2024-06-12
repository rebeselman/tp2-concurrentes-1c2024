mod robot;
mod ice_cream_container;
mod container_state;

use std::process::{Command, Child};
use std::env;
use std::thread;

const NUM_ROBOTS: usize = 4;

fn main() {
    // Este es el proceso principal de la Gestión de Pedidos
    let mut coordinador = launch_coordinador();
    let robots = launch_robots(NUM_ROBOTS);

    // Esperar a que todos los procesos terminen
    robots.into_iter().for_each(|mut robot| {
        let _ = robot.wait().expect("Robot process wasn't running");
    });
    let _ = coordinador.wait().expect("Coordinator process wasn't running");
}

fn launch_coordinador() -> Child {
    Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("coordinador")
        .spawn()
        .expect("Failed to launch coordinator process")
}

fn launch_robots(num_robots: usize) -> Vec<Child> {
    let mut robots: Vec<Child> = Vec::new();

    for i in 0..num_robots {
        let child = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("robot")
            .arg("--")
            .arg(format!("{}", i))
            .spawn()
            .expect("Failed to launch robot process");
        robots.push(child);
    }

    robots
}
