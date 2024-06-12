use payment_gateway::gateway;

fn main() {
    if let Err(err) = gateway::run() {
        eprintln!("An error occurred: {}", err);
    }
}
