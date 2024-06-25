use payment_gateway::gateway;

/// Gateway's entry point.
fn main() {
    if let Err(err) = gateway::run() {
        eprintln!("[Payment Gateway] An error occurred: {}", err);
    }
}
