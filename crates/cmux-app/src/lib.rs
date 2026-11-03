//! cmux-app: the gpui app. Everything but `main` lives here so it can be unit-tested.

pub mod actions;
pub mod assets;
#[cfg(target_os = "linux")]
pub mod blur;
pub mod chrome;
pub mod colors;
pub mod ime;
pub mod keys;
pub mod mouse;
pub mod style;
pub mod terminal_element;
pub mod terminal_view;
pub mod window_view;
pub mod workspace;
