//! Represents a screen state.

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
/// Describes the state of a screen. 
/// Active: 
/// Down:
/// Finished: the screen finished processing the orders. This means that the screen in charge should stop sending
/// pings messages.
pub enum ScreenState {
    Active(Option<usize>),
    Down(Option<usize>),
    Finished 
}