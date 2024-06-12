use std::thread;
use tokio::io;
use tokio::net::UdpSocket;

use crate::logger::Logger;
use crate::message;


const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";

async fn handle_messages(_logger: Logger) -> io::Result<()> {
    let server = UdpSocket::bind(PAYMENT_GATEWAY_IP).await?;
    println!("Payment Gateway listening on: {}", server.local_addr()?);

    loop {
        let mut buf = [0; 1024];
        let (len, addr) = server.recv_from(&mut buf).await?;

        let str_read = String::from_utf8_lossy(&buf[..len]);
        println!("Received message from {}: {}", addr, str_read);

        let mut message = message::deserialize_message(str_read.to_string()).unwrap();
        message.process_message();

        // Log state change
        // order.log_state_change(&mut file)?;
        
    }
}

pub fn run() -> io::Result<()> {
    let logger = Logger::new().unwrap();

    let handle = thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            if let Err(err) = handle_messages(logger).await {
                eprintln!("Error handling messages: {}", err);
            }
        });
    });

    // another thread for sending messages to Client Interfaces
    
    handle.join().unwrap();

    Ok(())
}
