use std::process::{Child, Command};


const NUM_ROBOTS: usize = 5;

fn main()  -> Result<(), Box<dyn std::error::Error>> {
    // Este es el proceso principal de la Gestión de Pedidos
    let robots: Vec<Child> = launch_robots(NUM_ROBOTS)?;

    // Esperar a que todos los procesos terminen
    robots.into_iter().for_each(|mut robot| {
        match robot.wait(){
            Ok(_) => println!("Robot process finished"),
            Err(e) => eprintln!("Error: {}", e)
        
        }// Update this line
    });
    Ok(())
}

fn launch_robots(num_robots: usize) -> Result<Vec<Child>, std::io::Error>{
    let mut robots: Vec<Child> = Vec::new();

    for i in 0..num_robots {
        let child = Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("robot")
            .arg("--")
            .arg(format!("{}", i))
            .arg(format!("{}", 0))
            .spawn()?;

        robots.push(child);
    }

    Ok(robots)
}
