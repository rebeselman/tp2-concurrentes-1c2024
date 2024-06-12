#[derive(Debug)]
pub enum OrderState {
    Pending,
    Authorized,
    Rejected,
}