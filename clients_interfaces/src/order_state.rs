//! Types of states of an order of ice cream 

use std::time::Instant;
#[derive(Copy, Clone, PartialEq, Eq, Hash)]

pub enum OrderState {
    Wait(Instant),
    Commit,
    Abort,
}
