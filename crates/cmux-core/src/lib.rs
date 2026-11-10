//! cmux-core: the UI-free model. M2: one workspace's split tree, panes and tabs.
//! M3: and the workspace list. M4: and the notification store.

mod geometry;
mod notifications;
mod tree;
mod workspaces;

pub use geometry::*;
pub use notifications::*;
pub use tree::*;
pub use workspaces::*;
