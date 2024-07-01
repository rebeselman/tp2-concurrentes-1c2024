use crate::logger::Logger;
use crate::messages::message;
use tokio::io;
use tokio::net::UdpSocket;

const PAYMENT_GATEWAY_IP: &str = "127.0.0.1:8081";
const LOG_FILE_PATH: &str = "log.txt";

/// Asynchronously handles incoming messages from the screens on a UDP socket,
/// processes them, sends responses back, and logs each message.
///
/// # Errors
///
/// Returns an `io::Error` if there's an issue with the socket operations or logging.
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
                    "[Payment Gateway] Received message '{}' from {}",
                    message.type_to_string(),
                    addr
                );

                let response = message.process();
                println!(
                    "[Payment Gateway] Sending message '{} {}' to {}",
                    message.get_order().id(),
                    String::from_utf8_lossy(&response)
                        .split('\n')
                        .last()
                        .unwrap_or("<Error getting message type>"),
                    addr
                );
                socket.send_to(&response, addr).await?;

                if let Err(e) = logger.log(&*message).await {
                    eprintln!("[Payment Gateway] Error logging message: {}", e);
                }
            }
            Err(e) => {
                eprintln!(
                    "[Payment Gateway] Error deserializing message from {}: {}",
                    addr, e
                );
            }
        }
    }
}

/// Creates an async logger and calls the main loop function over a Tokio runtime.
///
/// # Errors
///
/// Returns a `String` error message if there's an issue creating the Tokio runtime
/// or initializing the logger.
pub fn run() -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;

    runtime.block_on(async {
        let logger = Logger::new(LOG_FILE_PATH).await?;
        if let Err(err) = handle_messages(logger).await {
            eprintln!("[Payment Gateway] Error handling messages: {}", err);
        }

        Ok(())
    })
}
