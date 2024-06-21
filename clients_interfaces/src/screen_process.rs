//! Binary to process the orders using a screen given an id.
use std::env;
use clients_interfaces::screen::Screen;

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let args: Vec<String> = env::args().collect();
    let id: usize = args[1].parse()?;
    let mut screen = Screen::new(id)?;
    screen.process_orders()?;
    Ok(())
}