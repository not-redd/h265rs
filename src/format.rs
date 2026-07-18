use crate::GeometryError;

/// A chroma sampling structure from H.265 Table 6-1.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChromaFormat {
    /// `chroma_format_idc == 0`.
    Monochrome,
    /// `chroma_format_idc == 1`.
    Yuv420,
    /// `chroma_format_idc == 2`.
    Yuv422,
    /// `chroma_format_idc == 3`.
    Yuv444,
}

impl ChromaFormat {
    /// Returns `(SubWidthC, SubHeightC)` from Table 6-1.
    pub const fn subsampling(self) -> (u32, u32) {
        match self {
            Self::Monochrome | Self::Yuv444 => (1, 1),
            Self::Yuv420 => (2, 2),
            Self::Yuv422 => (2, 1),
        }
    }

    /// Returns the `chroma_format_idc` value.
    pub const fn idc(self) -> u8 {
        match self {
            Self::Monochrome => 0,
            Self::Yuv420 => 1,
            Self::Yuv422 => 2,
            Self::Yuv444 => 3,
        }
    }
}

/// The dimensions of one component/sample array.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaneDimension {
    /// Width in samples.
    pub width: u32,
    /// Height in samples.
    pub height: u32,
}

/// Source and decoded-picture format information from §6.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PictureFormat {
    /// Luma/sample-array width.
    pub width: u32,
    /// Luma/sample-array height.
    pub height: u32,
    /// Number of bits used by luma samples.
    pub bit_depth_luma: u8,
    /// Number of bits used by chroma samples.
    pub bit_depth_chroma: u8,
    /// Chroma sampling structure.
    pub chroma_format: ChromaFormat,
    /// Whether the three 4:4:4 planes are processed independently.
    pub separate_colour_plane: bool,
}

impl PictureFormat {
    /// Creates a validated picture format.
    pub fn new(
        width: u32,
        height: u32,
        bit_depth_luma: u8,
        bit_depth_chroma: u8,
        chroma_format: ChromaFormat,
        separate_colour_plane: bool,
    ) -> Result<Self, GeometryError> {
        if width == 0 || height == 0 {
            return Err(GeometryError::ZeroDimension);
        }
        if !(8..=16).contains(&bit_depth_luma) {
            return Err(GeometryError::InvalidBitDepth(bit_depth_luma));
        }
        if !(8..=16).contains(&bit_depth_chroma) {
            return Err(GeometryError::InvalidBitDepth(bit_depth_chroma));
        }
        if separate_colour_plane && !matches!(chroma_format, ChromaFormat::Yuv444) {
            return Err(GeometryError::SeparatePlanesRequire444);
        }
        Ok(Self {
            width,
            height,
            bit_depth_luma,
            bit_depth_chroma,
            chroma_format,
            separate_colour_plane,
        })
    }

    /// Returns the number of component arrays represented by the picture.
    pub const fn component_count(self) -> usize {
        match self.chroma_format {
            ChromaFormat::Monochrome => 1,
            ChromaFormat::Yuv420 | ChromaFormat::Yuv422 | ChromaFormat::Yuv444 => 3,
        }
    }

    /// Returns the dimensions of component `index` (`0 = Y`, `1 = Cb`, `2 = Cr`).
    pub const fn component_dimension(self, index: usize) -> Option<PlaneDimension> {
        if index >= self.component_count() {
            return None;
        }
        if index == 0 || self.separate_colour_plane {
            return Some(PlaneDimension {
                width: self.width,
                height: self.height,
            });
        }
        let (sub_width, sub_height) = self.chroma_format.subsampling();
        Some(PlaneDimension {
            width: ceil_div(self.width, sub_width),
            height: ceil_div(self.height, sub_height),
        })
    }

    /// Returns the number of prediction blocks for one prediction unit.
    pub const fn prediction_block_count(self) -> usize {
        if matches!(self.chroma_format, ChromaFormat::Yuv422) && !self.separate_colour_plane {
            5
        } else if matches!(self.chroma_format, ChromaFormat::Monochrome)
            || self.separate_colour_plane
        {
            1
        } else {
            3
        }
    }

    /// Returns the number of transform blocks for one transform unit.
    pub const fn transform_block_count(self) -> usize {
        self.prediction_block_count()
    }

    /// Returns the number of coding blocks for one coding unit.
    pub const fn coding_block_count(self) -> usize {
        if matches!(self.chroma_format, ChromaFormat::Monochrome) || self.separate_colour_plane {
            1
        } else {
            3
        }
    }
}

const fn ceil_div(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}
