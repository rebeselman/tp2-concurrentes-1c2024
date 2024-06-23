use tokio::io;
use tokio::net::UdpSocket;

use crate::logger::Logger;
use crate::message;

const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";


async fn handle_messages(_logger: Logger) -> io::Result<()> {
    let socket = UdpSocket::bind(PAYMENT_GATEWAY_IP).await?;
    println!("[Payment Gateway] Listening on: {}", socket.local_addr()?);

    loop {
        let mut buf = [0; 1024];
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let str_read = String::from_utf8_lossy(&buf[..len]).to_string();
        
        match message::deserialize_message(str_read) {
            Ok(message) => {
                println!("[Payment Gateway] Received message {} from {}", message.to_string(), addr);
                
                let response = message.process_message();
                socket.send_to(&response, addr).await?;

                // Log changes
                // message.log(logger);
            },
            Err(e) => {
                eprintln!("[Payment Gateway] Error deserializing message: {}", e);
            }
        }
    }
}

pub fn run() -> Result<(), String> {
    let logger = Logger::new()?;
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;

    runtime.block_on(async {
        if let Err(err) = handle_messages(logger).await {
            eprintln!("Error handling messages: {}", err);
        }
    });

    Ok(())
}
