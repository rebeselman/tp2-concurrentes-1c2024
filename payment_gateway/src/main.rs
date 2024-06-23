use payment_gateway::gateway;

fn main() {
    if let Err(err) = gateway::run() {
        eprintln!("[Payment Gateway] An error occurred: {}", err);
    }
}
