use crate::messages::message::Message;
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
};

const LOG_FILE_PATH: &str = "log.txt";

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
    pub async fn new() -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE_PATH)
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
