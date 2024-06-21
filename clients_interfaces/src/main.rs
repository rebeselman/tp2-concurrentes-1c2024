
use std::process::{exit, Command};

use clients_interfaces::generate_orders;
const NUMBER_SCREENS: u8 = 3;


fn main() {
    // create files of simulated orders
    generate_orders::generate_orders(NUMBER_SCREENS as u32).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        exit(1)
    });
 
    // create a screen process for each screen
    for id in 0..NUMBER_SCREENS {
        let child = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("screen_process")
            .arg(id.to_string())
            .spawn().unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1)
            });
        
        
       
        
    } 

   
   
}

