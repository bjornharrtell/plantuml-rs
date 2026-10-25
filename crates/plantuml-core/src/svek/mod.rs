//! Shapes shared with the graph layout engine (`svek`).

pub mod decorate;
pub mod extremity;
pub mod image;
pub mod node;
pub mod role_labels;
mod shapes;
pub mod types;

pub use shapes::{Boundary, Control, EntityDomain};
