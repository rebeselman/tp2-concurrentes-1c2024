//! Represents a screen state.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ScreenState {
    Active(Option<usize>),
    Down(Option<usize>),  
}