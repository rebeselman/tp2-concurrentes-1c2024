use tokio::io;
use tokio::net::UdpSocket;

use crate::logger::Logger;
use crate::message;

const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";

async fn handle_messages(mut logger: Logger) -> io::Result<()> {
    let socket = UdpSocket::bind(PAYMENT_GATEWAY_IP).await?;
    println!("[Payment Gateway] Listening on: {}", socket.local_addr()?);

    loop {
        let mut buf = [0; 1024];
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let str_read = String::from_utf8_lossy(&buf[..len]).to_string();

        match message::deserialize_message(str_read) {
            Ok(message) => {
                println!(
                    "[Payment Gateway] Received message {} from {}",
                    message.type_to_string(),
                    addr
                );

                match message.process() {
                    Ok(response) => {
                        socket.send_to(&response, addr).await?;
                    }
                    Err(e) => eprintln!("[Payment Gateway] Error processing message: {}", e),
                }

                if let Err(e) = logger.log(&*message).await {
                    eprintln!("[Payment Gateway] Error logging message: {}", e);
                }
            }
            Err(e) => {
                eprintln!(
                    "[Payment Gateway] Error deserializing message from {}: {} ",
                    addr, e
                );
            }
        }
    }
}

/// Gateway's entry point.
/// Creates an async logger and calls the main function over a tokio runtime.
pub fn run() -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;

    runtime.block_on(async {
        match Logger::new().await {
            Ok(logger) => {
                if let Err(err) = handle_messages(logger).await {
                    eprintln!("Error handling messages: {}", err);
                }
            }
            Err(e) => {
                eprintln!("Failed to initialize logger: {}", e);
                return Err(e);
            }
        }
        Ok(())
    })
}
