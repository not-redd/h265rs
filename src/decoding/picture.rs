use crate::{Block, ChromaFormat, PictureFormat};

/// A signed sample plane used by the reconstruction and in-loop filters.
///
/// Samples are signed internally so inverse transforms and filtering can be
/// performed without repeated conversions.  Values written through `set` are
/// not clipped; the process which owns the plane applies `clip_sample` at the
/// normative boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamplePlane {
    width: u32,
    height: u32,
    samples: Vec<i32>,
}

impl SamplePlane {
    /// Creates a plane initialized to `value`.
    pub fn new(width: u32, height: u32, value: i32) -> Self {
        let len = width.saturating_mul(height) as usize;
        Self {
            width,
            height,
            samples: vec![value; len],
        }
    }

    /// Creates a plane from row-major samples.
    pub fn from_samples(width: u32, height: u32, samples: Vec<i32>) -> Option<Self> {
        (samples.len() == width.saturating_mul(height) as usize).then_some(Self {
            width,
            height,
            samples,
        })
    }

    /// Plane width in samples.
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Plane height in samples.
    pub const fn height(&self) -> u32 {
        self.height
    }
    /// Returns all row-major samples.
    pub fn samples(&self) -> &[i32] {
        &self.samples
    }
    /// Returns all row-major samples mutably.
    pub fn samples_mut(&mut self) -> &mut [i32] {
        &mut self.samples
    }

    /// Reads a sample, returning `None` outside the plane.
    pub fn get(&self, x: i32, y: i32) -> Option<i32> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            None
        } else {
            Some(self.samples[y as usize * self.width as usize + x as usize])
        }
    }

    /// Reads with the coordinates clipped to the plane boundary.
    pub fn get_clipped(&self, x: i32, y: i32) -> i32 {
        let x = x.clamp(0, self.width.saturating_sub(1) as i32);
        let y = y.clamp(0, self.height.saturating_sub(1) as i32);
        self.samples[y as usize * self.width as usize + x as usize]
    }

    /// Writes a sample, ignoring out-of-range coordinates.
    pub fn set(&mut self, x: i32, y: i32, value: i32) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            let index = y as usize * self.width as usize + x as usize;
            self.samples[index] = value;
        }
    }

    /// Copies a rectangular block into this plane, clipping the destination.
    pub fn write_block(&mut self, block: Block, values: &[i32]) {
        for y in 0..block.height {
            for x in 0..block.width {
                let source = y as usize * block.width as usize + x as usize;
                if source < values.len() {
                    self.set(
                        block.x as i32 + x as i32,
                        block.y as i32 + y as i32,
                        values[source],
                    );
                }
            }
        }
    }

    /// Reads a rectangular block, clipping out-of-picture samples.
    pub fn read_block(&self, block: Block) -> Vec<i32> {
        let mut values = Vec::with_capacity(block.width as usize * block.height as usize);
        for y in 0..block.height {
            for x in 0..block.width {
                values.push(self.get_clipped(block.x as i32 + x as i32, block.y as i32 + y as i32));
            }
        }
        values
    }
}

/// The reconstructed picture consisting of one or three component planes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedPicture {
    /// Picture format and component geometry.
    pub format: PictureFormat,
    planes: [Option<SamplePlane>; 3],
}

impl DecodedPicture {
    /// Creates a zero-filled decoded picture.
    pub fn new(format: PictureFormat) -> Self {
        let planes = std::array::from_fn(|index| {
            format
                .component_dimension(index)
                .map(|dimension| SamplePlane::new(dimension.width, dimension.height, 0))
        });
        Self { format, planes }
    }

    /// Returns a component plane.
    pub fn plane(&self, component: usize) -> Option<&SamplePlane> {
        self.planes.get(component).and_then(Option::as_ref)
    }

    /// Returns a mutable component plane.
    pub fn plane_mut(&mut self, component: usize) -> Option<&mut SamplePlane> {
        self.planes.get_mut(component).and_then(Option::as_mut)
    }

    /// Replaces a component plane after checking its dimensions.
    pub fn set_plane(&mut self, component: usize, plane: SamplePlane) -> bool {
        let Some(expected) = self.format.component_dimension(component) else {
            return false;
        };
        if expected.width != plane.width || expected.height != plane.height {
            return false;
        }
        self.planes[component] = Some(plane);
        true
    }

    /// Returns the component's bit depth.
    pub const fn bit_depth(&self, component: usize) -> u8 {
        if component == 0 || self.format.separate_colour_plane {
            self.format.bit_depth_luma
        } else {
            self.format.bit_depth_chroma
        }
    }

    /// Returns chroma subsampling factors.
    pub const fn subsampling(&self) -> (u32, u32) {
        self.format.chroma_format.subsampling()
    }

    /// Returns the chroma format, including the monochrome case.
    pub const fn chroma_format(&self) -> ChromaFormat {
        self.format.chroma_format
    }
}
