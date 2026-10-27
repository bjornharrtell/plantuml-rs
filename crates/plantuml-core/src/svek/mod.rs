//! Shapes shared with the graph layout engine (`svek`).

pub mod bibliotekon;
pub mod builder;
pub mod cluster;
pub mod cluster_header;
pub mod decorate;
pub mod edge;
pub mod extremity;
pub mod image;
pub mod image_chen;
pub mod image_class;
pub mod image_description;
pub mod image_misc;
pub mod image_note;
pub mod image_object;
pub mod image_port;
pub mod image_state;
pub mod image_tips;
pub mod node;
pub mod note_link;
pub(crate) mod ports;
pub mod role_labels;
pub mod rounded_container;
mod shapes;
pub mod types;

pub use shapes::{Boundary, Control, EntityDomain};
