use std::fmt;

/// Errors raised while constructing Clause 6 geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GeometryError {
    /// A required dimension was zero.
    ZeroDimension,
    /// A bit depth was outside H.265's 8..=16 range.
    InvalidBitDepth(u8),
    /// Separate colour planes are only defined for 4:4:4 in this clause.
    SeparatePlanesRequire444,
    /// A CTB or minimum transform-block size is invalid.
    InvalidBlockSize,
    /// Tile widths or heights do not cover the picture.
    TileDimensionsDoNotCoverPicture,
    /// Tile dimensions contain a zero-sized tile.
    ZeroSizedTile,
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => write!(f, "picture dimensions must be non-zero"),
            Self::InvalidBitDepth(depth) => write!(f, "invalid bit depth {depth}; expected 8..=16"),
            Self::SeparatePlanesRequire444 => {
                write!(f, "separate colour planes require 4:4:4 sampling")
            }
            Self::InvalidBlockSize => {
                write!(f, "block sizes must be powers of two and properly nested")
            }
            Self::TileDimensionsDoNotCoverPicture => {
                write!(f, "tile dimensions do not cover the complete CTB picture")
            }
            Self::ZeroSizedTile => write!(f, "tile dimensions must be non-zero"),
        }
    }
}

impl std::error::Error for GeometryError {}
