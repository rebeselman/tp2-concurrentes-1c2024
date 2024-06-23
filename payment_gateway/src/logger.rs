use tokio::{fs::{File, OpenOptions}, io::AsyncWriteExt};

use crate::message::Message;

const LOG_FILE_PATH: &str = "log.txt";


pub struct Logger {
    file: File
}

impl Logger {
    pub async fn new() -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE_PATH)
            .await
            .map_err(|e| format!("Error opening log file: {}", e))?;
        Ok(Logger { file })
    }

    pub async fn log(&mut self, message: &dyn Message) -> Result<(), String> {
        let log_entry = message.log()?;
        self.file
            .write_all(log_entry.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

}