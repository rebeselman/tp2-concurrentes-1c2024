use crate::messages::message::Message;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

/// A struct representing a logger that asynchronously writes log entries to a file.
pub struct Logger {
    file: File,
}

impl Logger {
    /// Creates a new `Logger` instance, opening (or creating) the log file for appending.
    ///
    /// # Returns
    ///
    /// - `Ok(Logger)`: A `Logger` instance with the log file opened.
    /// - `Err(String)`: An error message if the log file could not be opened.
    pub async fn new(file_path: &str) -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .await
            .map_err(|e| format!("Error opening log file: {}", e))?;
        Ok(Logger { file })
    }

    /// Logs a message by writing its log entry to the log file.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: If the message was successfully logged.
    /// - `Err(String)`: An error message if there was an issue writing to the log file.
    pub async fn log(&mut self, message: &dyn Message) -> Result<(), String> {
        let log_entry = message.log_entry()?;
        self.file
            .write_all(log_entry.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{abort::Abort, commit::Commit, prepare::Prepare};
    use orders::order::Order;
    use tokio::fs;

    #[tokio::test]
    async fn test_create_logger() {
        let file_path = "test_create_logger.txt";

        // Ensure the file does not exist before the test
        if fs::metadata(file_path).await.is_ok() {
            fs::remove_file(file_path).await.unwrap();
        }

        let _logger = Logger::new(file_path).await;
        assert!(fs::metadata(file_path).await.is_ok());

        fs::remove_file(file_path).await.unwrap()
    }

    #[tokio::test]
    async fn test_log_abort_message() {
        let file_path = "test_log_abort.txt";
        let mut logger = Logger::new(file_path).await.unwrap();

        // Clear the file before writing
        fs::write(file_path, "").await.unwrap();

        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Abort::new(order);
        logger.log(&message).await.unwrap();

        let content = fs::read_to_string(file_path).await.unwrap();
        let log_entry =
            r#"abort {"order_id":9,"client_id":25,"credit_card":"0000111122223333","items":[]}"#;
        assert_eq!(content, format!("{}\n", log_entry));

        fs::remove_file(file_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_log_commit_message() {
        let file_path = "test_log_commit.txt";
        let mut logger = Logger::new(file_path).await.unwrap();

        // Clear the file before writing
        fs::write(file_path, "").await.unwrap();

        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Commit::new(order);
        logger.log(&message).await.unwrap();

        let content = fs::read_to_string(file_path).await.unwrap();
        let log_entry =
            r#"commit {"order_id":9,"client_id":25,"credit_card":"0000111122223333","items":[]}"#;
        assert_eq!(content, format!("{}\n", log_entry));

        fs::remove_file(file_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_log_prepare_message() {
        let file_path = "test_log_prepare.txt";
        let mut logger = Logger::new(file_path).await.unwrap();

        // Clear the file before writing
        fs::write(file_path, "").await.unwrap();

        let order = Order::new(9, 25, "0000111122223333".to_string(), Vec::new());
        let message = Prepare::new(order);
        logger.log(&message).await.unwrap();

        let content = fs::read_to_string(file_path).await.unwrap();
        let log_entry =
            r#"prepare {"order_id":9,"client_id":25,"credit_card":"0000111122223333","items":[]}"#;
        assert_eq!(content, format!("{}\n", log_entry));

        fs::remove_file(file_path).await.unwrap();
    }
}
