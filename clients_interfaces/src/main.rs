use std::process::{exit, Child, Command};

use clients_interfaces::generate_orders;
const NUMBER_SCREENS: u8 = 3;

fn main() {
    // create files of simulated orders
    generate_orders::generate_orders(NUMBER_SCREENS as u32).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        exit(1)
    });

    let screens = launch_screens();

    screens.into_iter().for_each(|mut screen| {
        let _ = screen.wait().unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            exit(1)
        });
    });
}

fn launch_screens() -> Vec<Child> {
    let mut screens: Vec<Child> = Vec::new();

    // create a screen process for each screen
    for id in 0..NUMBER_SCREENS {
        let child = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("screen_process")
            .arg(id.to_string())
            .spawn()
            .unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1)
            });
        screens.push(child);
    }

    screens
}
