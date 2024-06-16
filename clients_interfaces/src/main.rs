
use std::process::{exit, Command};
const NUMBER_SCREENS: u8 = 3;


fn main() {
    for id in 0..NUMBER_SCREENS {
        let mut child = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("screen_process")
            .arg(id.to_string())
            .spawn().unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                exit(1)
            });
        
        
        child.wait().unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            exit(1)
        });
        
    }  
   
}

