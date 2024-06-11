//! Types of states of an order of ice cream 
#[derive(Copy, Clone, PartialEq, Eq, Hash)]

pub enum OrderState {
    Wait,
    Commit,
    Abort,
}
