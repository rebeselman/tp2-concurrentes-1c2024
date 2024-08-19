use actix::prelude::Actor;
use clients_interfaces::{screen::Screen, screen_message::ScreenMessage};
use std::{env, error::Error};
const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:8081";
#[actix_rt::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let id: usize = args[1].parse()?;
    let mut screen = Screen::new(id)?;
    let screen_cloned = screen.clone_screen()?;
    let screen_actor = screen_cloned.start();
    // estaria bueno reemplazar por algo asi:

    while !screen.is_finished() {
        let mut buf = [0; 1024];
        let (size, from) = screen.socket.recv_from(&mut buf)?;
        let message = String::from_utf8_lossy(&buf[..size]);
        let mut parts = message.split('\n');

        let response = parts.next().ok_or("No response")?;

        match response {
            "ready" | "abort" | "finished" | "keepalive" => {
                let mut order_management_ip = screen
                    .order_management_ip
                    .lock()
                    .map_err(|e| e.to_string())?;
                if from != *order_management_ip && from.to_string() != PAYMENT_GATEWAY_IP {
                    // change to screen.order_management_ip
                   
                    *order_management_ip = from;
                }
                drop(order_management_ip);
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
    println!("Screen {} finished COMPLETELY", screen.id());

    Ok(())
}
