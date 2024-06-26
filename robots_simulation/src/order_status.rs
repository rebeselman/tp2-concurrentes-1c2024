#[derive(Clone, Eq, PartialEq)]
pub enum OrderStatus {
    Pending,
    CompletedButNotCommited,
    CommitReceived,
    Completed,
}