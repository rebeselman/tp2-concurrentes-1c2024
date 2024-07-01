
#[derive(PartialEq)]
pub enum ElectionState {
    StartingElection,
    Candidate,
    Follower,
    None,
}