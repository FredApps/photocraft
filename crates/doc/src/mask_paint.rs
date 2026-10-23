//! Editable brush layers composited into a layer mask. The flattened surface remains
//! available to existing compositors and exports.
use crate::LayerMask;
use photocraft_raster::Surface;

/// One transparent grayscale paint layer, stored as RGBA for independent coverage.
#[derive(Clone, Debug, PartialEq)]
pub struct Paint {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub surface: Surface,
}

/// Original mask plus ordered paint layers (bottom to top).
#[derive(Clone, Debug, PartialEq)]
pub struct Stack {
    pub base: Surface,
    pub layers: Vec<Paint>,
    pub active: usize,
}

impl LayerMask {
    /// The selected brush layer, or the ordinary mask when no stack exists.
    pub fn paint_surface(&self) -> &Surface {
        self.paint.as_ref().and_then(|s| s.layers.get(s.active)).map_or(&self.surface, |p| &p.surface)
    }

    /// Mutable brush target. The caller refreshes the flattened mask after painting.
    pub fn paint_surface_mut(&mut self) -> &mut Surface {
        match self.paint.as_mut().and_then(|s| s.layers.get_mut(s.active)) {
            Some(p) => &mut p.surface,
            None => &mut self.surface,
        }
    }
}
