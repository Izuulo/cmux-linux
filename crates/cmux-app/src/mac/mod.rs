//! The macOS (Tahoe) look: tokens and components every surface is built from (M14 plan K1).
//! New UI uses these instead of its own colours, radii, shadows or timings.

pub mod controls;
pub mod fade;
pub mod glass;
pub mod menu;
pub mod popover;
pub mod tokens;

pub use fade::Fade;
pub use glass::glass_panel;
pub use menu::{menu, menu_height, menu_item, menu_separator};
pub use popover::{ArrowPts, arrow_points, popover};
pub use tokens::{Theme, alpha, metric, motion, popover_shadow, scale_alpha};
