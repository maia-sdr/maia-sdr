use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Colormap {
    Turbo,
    Viridis,
    Inferno,
}

impl Colormap {
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
