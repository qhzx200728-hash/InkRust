// Nguon goc (Source): Rnote (crates/rnote-compose/src/transformable.rs)
// Commit hash (reference/rnote): 83a76367dbd3d6c461c10553c45231799a25ea4b
// Giay phep (License): GPL-3.0-or-later (GNU General Public License v3.0 or later)

// Imports
use p2d::math::Vector2;

/// Trait for types that can be (geometrically) transformed.
pub trait Transformable {
    /// Translate (as in moves) by the given offset.
    fn translate(&mut self, offset: Vector2);
    /// Rotate by the given angle (in radians).
    fn rotate(&mut self, angle: f64, center: Vector2);
    /// Scale by the given scale-factor.
    fn scale(&mut self, scale: Vector2);
}
