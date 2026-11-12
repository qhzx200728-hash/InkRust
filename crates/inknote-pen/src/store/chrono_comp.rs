// Nguon goc (Source): Rnote (crates/rnote-engine/src/store/chrono_comp.rs)
// Commit hash (reference/rnote): 83a76367dbd3d6c461c10553c45231799a25ea4b
// Giay phep (License): GPL-3.0-or-later (GNU General Public License v3.0 or later)

// Imports
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq)]
#[serde(rename = "stroke_layer")]
pub enum StrokeLayer {
    #[serde(rename = "user_layer", alias = "UserLayer")]
    UserLayer(u32),
    #[serde(rename = "highlighter", alias = "Highlighter")]
    Highlighter,
    #[serde(rename = "image", alias = "Image")]
    Image,
    #[serde(rename = "document", alias = "Document")]
    Document,
}

impl Default for StrokeLayer {
    fn default() -> Self {
        Self::UserLayer(0)
    }
}

impl PartialEq for StrokeLayer {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::UserLayer(l0), Self::UserLayer(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl PartialOrd for StrokeLayer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StrokeLayer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let key = |layer: &StrokeLayer| match layer {
            StrokeLayer::Document => (0_u8, 0_u32),
            StrokeLayer::Image => (1_u8, 0_u32),
            StrokeLayer::UserLayer(layer_no) => (2_u8, *layer_no),
            StrokeLayer::Highlighter => (3_u8, 0_u32),
        };

        key(self).cmp(&key(other))
    }
}
