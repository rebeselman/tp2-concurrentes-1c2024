use std::fs::{File, OpenOptions};

const LOG_FILE_PATH: &str = "log.txt";

pub struct Logger {
    _file: File
}

impl Logger {
    pub fn new() -> Result<Self, String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE_PATH)
            .map_err(|e| format!("Error opening log file: {}", e))?;
        Ok(Logger { _file: file })
    }

    // fn log_state_change(&self, file: &mut std::fs::File) -> io::Result<()> {
    //     writeln!(file, "Order {:?} - State: {:?}", self.order_id, self.state)
    // }

}