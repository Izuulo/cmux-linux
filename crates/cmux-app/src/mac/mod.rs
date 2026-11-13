//! The macOS (Tahoe) look: tokens and components every surface is built from (M14 plan K1).
//! New UI uses these instead of its own colours, radii, shadows or timings.

pub mod fade;
pub mod glass;
pub mod popover;
pub mod tokens;

pub use fade::Fade;
pub use glass::glass_panel;
pub use popover::{ArrowPts, arrow_points, popover};
pub use tokens::{Theme, metric, motion, popover_shadow};
