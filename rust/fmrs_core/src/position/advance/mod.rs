pub mod advance;
pub mod attack_prevent;
mod black;
mod common;
mod free;
mod options;
pub mod pinned;
mod white;

pub use common::checked;
pub use free::{is_legal_mate, legal_movements};
pub use options::{AdvanceError, AdvanceOptions, AdvanceResult};
