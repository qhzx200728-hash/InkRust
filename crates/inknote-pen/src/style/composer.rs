// Nguon goc (Source): Rnote (crates/rnote-compose/src/style/composer.rs)
// Commit hash (reference/rnote): 83a76367dbd3d6c461c10553c45231799a25ea4b
// Giay phep (License): GPL-3.0-or-later (GNU General Public License v3.0 or later)

// Imports
use p2d::bounding_volume::Aabb;

/// Trait for types can be composed and drawn with a style.
pub trait Composer<O>
where
    O: std::fmt::Debug + Clone,
{
    /// Bounds of the composed shape.
    fn composed_bounds(&self, options: &O) -> Aabb;

    /// Composes and draws the type onto the context, applying the style options to it.
    fn draw_composed(&self, cx: &mut impl piet::RenderContext, options: &O);
}
