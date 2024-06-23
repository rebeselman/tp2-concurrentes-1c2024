use tokio::io;
use std::net::UdpSocket;

use crate::logger::Logger;
use crate::message;

const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";


fn handle_messages(_logger: Logger) -> io::Result<()> {
    let socket = UdpSocket::bind(PAYMENT_GATEWAY_IP)?;
    println!("[Payment Gateway] Listening on: {}", socket.local_addr()?);

    loop {
        let mut buf = [0; 1024];
        let (len, addr) = socket.recv_from(&mut buf)?;

        let str_read = String::from_utf8_lossy(&buf[..len]);
        println!("[Payment Gateway] Received message from {}: {}", addr, str_read);

        let mut message = message::deserialize_message(str_read.to_string()).unwrap();
        message.process_message(&socket, addr);

        // Log changes
        // message.log(logger);
        
    }
}

pub fn run() -> io::Result<()> {
    let logger = Logger::new().unwrap();

    if let Err(err) = handle_messages(logger) {
        eprintln!("Error handling messages: {}", err);
    }

    // let runtime = tokio::runtime::Runtime::new().unwrap();
    // runtime.block_on(async {
    //     if let Err(err) = handle_messages(logger).await {
    //         eprintln!("Error handling messages: {}", err);
    //     }
    // });
   
    Ok(())
}
