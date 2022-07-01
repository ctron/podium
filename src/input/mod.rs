use crate::input::key::Key;

pub mod events;
pub mod key;

// inputs/mod.rs
pub enum InputEvent {
    /// An input event occurred.
    Input(Key),
    /// Redraw application
    Render,
    /// Exit application
    Quit,
}
