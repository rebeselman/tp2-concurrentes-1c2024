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
async fn handle_messages(addr: &str, mut logger: Logger) -> io::Result<()> {
    let socket = UdpSocket::bind(addr).await?;
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
        if let Err(err) = handle_messages(PAYMENT_GATEWAY_IP, logger).await {
            eprintln!("[Payment Gateway] Error handling messages: {}", err);
        }

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read_to_string;
    use tokio::{
        task,
        time::{sleep, Duration},
    };

    #[tokio::test]
    async fn test_handle_abort_message() {
        let file_path = "test_handle_abort.txt";
        let screen_addr = "127.0.0.1:12340";

        if std::path::Path::new(file_path).exists() {
            std::fs::remove_file(file_path).unwrap();
        }
        let logger = Logger::new(file_path).await.unwrap();

        let handler = task::spawn(async move {
            handle_messages(PAYMENT_GATEWAY_IP, logger).await.unwrap();
        });

        // Allow the handler to start
        sleep(Duration::from_millis(100)).await;

        let screen_socket = UdpSocket::bind(screen_addr).await.unwrap();
        screen_socket.send_to(b"abort\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}", PAYMENT_GATEWAY_IP).await.unwrap();

        let mut buf = [0; 1024];
        let (len, _src) = screen_socket.recv_from(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf[..len]).to_string();
        assert_eq!(response, "9\nabort");

        // Give some time for logging
        sleep(Duration::from_millis(100)).await;
        let log_contents = read_to_string(file_path).unwrap();
        assert_eq!(log_contents, "abort {\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}\n");

        handler.abort();
        std::fs::remove_file(file_path).unwrap();
    }

    #[tokio::test]
    async fn test_handle_commit_message() {
        let file_path = "test_handle_commit.txt";
        let screen_addr = "127.0.0.1:12341";

        if std::path::Path::new(file_path).exists() {
            std::fs::remove_file(file_path).unwrap();
        }
        let logger = Logger::new(file_path).await.unwrap();

        let handler = task::spawn(async move {
            handle_messages(&PAYMENT_GATEWAY_IP.replace(":8081", ":8082"), logger)
                .await
                .unwrap();
        });

        // Allow the handler to start
        sleep(Duration::from_millis(100)).await;

        let screen_socket = UdpSocket::bind(screen_addr).await.unwrap();
        screen_socket.send_to(b"commit\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}", PAYMENT_GATEWAY_IP.replace(":8081", ":8082")).await.unwrap();

        let mut buf = [0; 1024];
        let (len, _src) = screen_socket.recv_from(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf[..len]).to_string();
        assert_eq!(response, "9\nfinished");

        // Give some time for logging
        sleep(Duration::from_millis(100)).await;
        let log_contents = read_to_string(file_path).unwrap();
        assert_eq!(log_contents, "commit {\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}\n");

        handler.abort();
        std::fs::remove_file(file_path).unwrap();
    }

    #[tokio::test]
    async fn test_handle_prepare_message() {
        let file_path = "test_handle_prepare.txt";
        let screen_addr = "127.0.0.1:12342";

        if std::path::Path::new(file_path).exists() {
            std::fs::remove_file(file_path).unwrap();
        }
        let logger = Logger::new(file_path).await.unwrap();

        let handler = task::spawn(async move {
            handle_messages(&PAYMENT_GATEWAY_IP.replace(":8081", ":8083"), logger)
                .await
                .unwrap();
        });

        // Allow the handler to start
        sleep(Duration::from_millis(100)).await;

        let screen_socket = UdpSocket::bind(screen_addr).await.unwrap();
        screen_socket.send_to(b"prepare\n{\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}", PAYMENT_GATEWAY_IP.replace(":8081", ":8083")).await.unwrap();

        let mut buf = [0; 1024];
        let (len, _src) = screen_socket.recv_from(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf[..len]).to_string();
        assert_eq!(response, "9\nready");

        // Give some time for logging
        sleep(Duration::from_millis(100)).await;
        let log_contents = read_to_string(file_path).unwrap();
        assert_eq!(log_contents, "prepare {\"order_id\":9,\"client_id\":25,\"credit_card\":\"0000111122223333\",\"items\":[]}\n");

        handler.abort();
        std::fs::remove_file(file_path).unwrap();
    }
}
