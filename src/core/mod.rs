// src/core/mod.rs
// Domain model: images, sessions, ratings, and the cull state machine.

pub mod analysis;
pub mod burst;
pub mod caption;
pub mod develop;
pub mod exif;
pub mod export;
pub mod image;
pub mod import;
pub mod pdf;
pub mod session;
pub mod thumbnailer;
pub mod watcher;
pub mod xmp;
pub mod zine;

pub use image::{CullState, ImageEntry, ImageFormat, ImageId};
pub use session::{Session, SessionId};
