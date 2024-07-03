use clients_interfaces::{screen::Screen, screen_message::ScreenMessage};
use std::{env, error::Error};
use actix::prelude::Actor;

#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let id: usize = args[1].parse()?;
    let mut screen = Screen::new(id)?;
    let screen_cloned = screen.clone()?;
    let screen_actor = screen_cloned.start();
    // estaria bueno reemplazar por algo asi:

    while !screen.is_finished() {
        let mut buf = [0; 1024];
        let (size, from) = screen.socket.recv_from(&mut buf)?;
        let message = String::from_utf8_lossy(&buf[..size]);
        let mut parts = message.split('\n');

        let response = parts.next().ok_or("No response")?;
        
        
        match response {
            "ready" | "abort" | "finished" |"keepalive"=> {
                let order_id = parts.next().ok_or("No order id")?.parse::<usize>()?;
                screen.handle_message(response, from.to_string(), order_id)?;
            }
            
            "screen" => {
                let message: ScreenMessage =
                    serde_json::from_str(parts.next().ok_or("No message")?)?;
                screen_actor.send(message).await?;
            }
            _ => {
                println!("[SCREEN {}] ??? received", screen.id());
            }
        }
    }



    Ok(())

}