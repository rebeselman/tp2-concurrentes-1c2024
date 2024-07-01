
#[derive(PartialEq)]
pub enum ElectionState {
    Leader,
    StartingElection,
    Candidate,
    Follower,
    None,
}