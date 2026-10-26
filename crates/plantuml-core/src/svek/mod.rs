//! Shapes shared with the graph layout engine (`svek`).

pub mod bibliotekon;
pub mod builder;
pub mod cluster;
pub mod decorate;
pub mod edge;
pub mod extremity;
pub mod image;
pub mod image_class;
pub mod image_misc;
pub mod image_note;
pub mod node;
pub mod note_link;
pub mod role_labels;
mod shapes;
pub mod types;

pub use shapes::{Boundary, Control, EntityDomain};
