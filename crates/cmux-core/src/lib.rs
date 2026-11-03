//! cmux-core: the UI-free model. M2: one workspace's split tree, panes and tabs.
//! M3: and the workspace list.

mod geometry;
mod tree;
mod workspaces;

pub use geometry::*;
pub use tree::*;
pub use workspaces::*;
