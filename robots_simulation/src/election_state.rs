
#[derive(PartialEq, Clone)]
pub enum ElectionState {
    StartingElection,
    Candidate,
    Follower,
    None,
}