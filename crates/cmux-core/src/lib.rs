//! cmux-core: the UI-free model. M2: one workspace's split tree, panes and tabs.
//! M3: and the workspace list. M4: and the notification store.
//! M8: and workspace groups. M9: and agent messages.
//! M10: and the saved shape of both.

mod feed;
mod geometry;
mod groups;
mod messages;
mod notifications;
mod session;
mod tree;
mod workspaces;

pub use feed::*;
pub use geometry::*;
pub use groups::*;
pub use messages::*;
pub use notifications::*;
pub use session::*;
pub use tree::*;
pub use workspaces::*;
