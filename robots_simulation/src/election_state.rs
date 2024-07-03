
#[derive(PartialEq, Clone, Debug)]
pub enum ElectionState {
    StartingElection,
    Candidate,
    Follower,
    None,
}