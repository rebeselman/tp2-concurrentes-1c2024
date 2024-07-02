#[derive(Clone, Eq, PartialEq, Debug)]
pub enum OrderStatus {
    Pending,
    CompletedButNotCommited,
    CommitReceived,
    Completed,
    Aborted,
}
