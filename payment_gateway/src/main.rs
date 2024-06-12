use payment_gateway::logger::Logger;
use payment_gateway::order::Order;
use payment_gateway::order_state::OrderState;
use rand::Rng;
use tokio::io;
use tokio::net::UdpSocket;

const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:1024";

#[tokio::main]
async fn main() -> io::Result<()> {
    let server = UdpSocket::bind(PAYMENT_GATEWAY_IP).await?;
    println!("Payment Gateway listening on: {}", server.local_addr()?);

    let logger = Logger::new();
    loop {
        let mut buf = [0; 1024];
        let (len, addr) = server.recv_from(&mut buf).await?;

        let message = String::from_utf8_lossy(&buf[..len]);
        println!("Received message from {}: {}", addr, message);

        if message.starts_with("prepare") {
            // Parse order data from the message
            let order_data: Vec<&str> = message.split(',').collect();
            let order_id = order_data[1].trim().to_string();
            let client_id = order_data[2].trim().to_string();
            let credit_card_number = order_data[3].trim().to_string();

            // Process payment
            let mut order = Order::new(
                order_id.clone(),
                client_id.clone(),
                credit_card_number.clone(),
            );
            let authorize = rand::thread_rng().gen_bool(0.9);
            if authorize {
                order.update_state(OrderState::Authorized);
                server.send_to(b"ready", &addr).await?;
            } else {
                order.update_state(OrderState::Rejected);
                server.send_to(b"abort", &addr).await?;
            }

            // Log state change
            // order.log_state_change(&mut file)?;
        }
    }
}
