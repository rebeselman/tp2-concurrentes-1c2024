use std::net::UdpSocket;
use std::io;
use serde::{Deserialize, Serialize};
use gateway_payment::order::Order;


fn main() -> io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:8081")?; // Bind to the same address as the payment gateway
    let mut buf = [0; 1024];

    loop {
        println!("Waiting for a message from the screen...");
        let (amt, src) = socket.recv_from(&mut buf)?;

        println!("Received a message from the screen: {:?}", &buf[..amt]);

        // Convert the received bytes into a string
        let received_message = String::from_utf8_lossy(&buf[..amt]);
        let message = received_message.trim();
        println!("Received message: {}", message);
        let mut parts = message.split('\n');
        // Save first part of parts as String and last part as order object from json
        let message_type = parts.next().unwrap();
        // Parse order to object with json serde
        let order: Order = serde_json::from_str(&parts.next().unwrap()).unwrap();

        println!("Order ID: {}", order.order_id);

        // Check if the received message is "prepare"
        if message_type == "prepare" {
            println!("Preparing order: {}", order.order_id);
            let message = format!("{}\nready", order.order_id).into_bytes();
            // Send "ready" message back to the screen
            socket.send_to(&message, &src)?;
        }

        if message_type == "commit" {
            println!("Preparing order: {}", order.order_id);
            let message = format!("{}\nfinished", order.order_id).into_bytes();
            println!("len: {}", message.len());
            // Send "ready" message back to the screen
            socket.send_to(&message, &src)?;
            println!("Next order id: {}", order.order_id);
        }
    }
}