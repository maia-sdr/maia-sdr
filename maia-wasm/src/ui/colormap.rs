//! Colormaps.
//!
//! This module defines the colormaps that are supported by the maia-wasm
//! waterfall.

use serde::{Deserialize, Serialize};

/// Waterfall colormap.
///
/// This enum lists the supported waterfall colormaps.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Colormap {
    /// Turbo colormap.
    Turbo,
    /// Viridis colormap.
    Viridis,
    /// Inferno colormap.
    Inferno,
}

impl Colormap {
    /// Returns the colormap as a slice.
    ///
    /// The format of the slice is 8-bit RGB as a flattened array. Usually the
    /// length of the colormap is 255 RGB pixels, since it is indexed by an
    /// 8-bit integer, but this need not be the case.
    pub fn colormap_as_slice(&self) -> &[u8] {
        match self {
            Colormap::Turbo => &crate::colormap::turbo::COLORMAP,
            Colormap::Viridis => &crate::colormap::viridis::COLORMAP,
            Colormap::Inferno => &crate::colormap::inferno::COLORMAP,
        }
    }
}

impl std::str::FromStr for Colormap {
    type Err = ();

    fn from_str(s: &str) -> Result<Colormap, ()> {
        Ok(match s {
            "Turbo" => Colormap::Turbo,
            "Viridis" => Colormap::Viridis,
            "Inferno" => Colormap::Inferno,
            _ => return Err(()),
        })
    }
}

impl std::fmt::Display for Colormap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Colormap::Turbo => "Turbo",
                Colormap::Viridis => "Viridis",
                Colormap::Inferno => "Inferno",
            }
        )
    }
}
