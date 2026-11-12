// Nguon goc (Source): Rnote (crates/rnote-compose/src/lib.rs)
// Commit hash (reference/rnote): 83a76367dbd3d6c461c10553c45231799a25ea4b
// Giay phep (License): GPL-3.0-or-later (GNU General Public License v3.0 or later)

#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![allow(clippy::single_match)]

//! the rnote-compose crate provides rnote with building blocks for creating, styling, composing, drawing, transforming shapes and paths.

// Modules
/// module for shape builders
pub mod builders;
/// colors
pub mod color;
/// constraints
pub mod constraints;
/// Event result.
pub mod eventresult;
/// module for extension traits for foreign types
pub mod ext;
/// module for pen events
pub mod penevent;
/// module for pen paths
pub mod penpath;
/// utilities for serializing / deserializing
pub mod serialize;
/// module for shapes
pub mod shapes;
/// module for splitorder
pub mod splitorder;
/// module for styles, that can be applied onto shapes
pub mod style;
/// module for transformationable trait
pub mod transformable;
/// other misc utilities
pub mod utils;

// Re-exports
pub use color::Color;
pub use constraints::Constraints;
pub use eventresult::EventResult;
pub use penevent::PenEvent;
pub use penpath::PenPath;
pub use shapes::Shape;
pub use splitorder::SplitOrder;
pub use style::Style;
pub use transformable::Transformable;

// Renames
extern crate parry2d_f64 as p2d;

// --- InkNote: phan them (khong co trong ban goc Rnote), xem docs/03-port-rnote.md ---
// Cho phep brushconfig.rs (copy nguyen van tu rnote-engine) giu nguyen duong dan `rnote_compose::...`.
extern crate self as rnote_compose;
/// Cau hinh but (Marker/Solid), copy tu rnote-engine.
pub mod brushconfig;
/// Chi chua StrokeLayer (copy tu rnote-engine) vi brushconfig.rs can.
pub mod store {
    /// chrono_comp (chi StrokeLayer)
    pub mod chrono_comp;
}

