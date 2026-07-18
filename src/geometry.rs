use crate::{GeometryError, PictureFormat};

/// Picture-level CTB geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PictureGeometry {
    /// Luma picture format.
    pub format: PictureFormat,
    /// CTB width and height in luma samples.
    pub ctb_size: u32,
    /// Minimum transform-block width and height in luma samples.
    pub min_tb_size: u32,
}

impl PictureGeometry {
    /// Creates a geometry after validating the CTB/minimum-TB relationship.
    pub fn new(
        format: PictureFormat,
        ctb_size: u32,
        min_tb_size: u32,
    ) -> Result<Self, GeometryError> {
        if !is_power_of_two(ctb_size)
            || !is_power_of_two(min_tb_size)
            || min_tb_size > ctb_size
            || !ctb_size.is_multiple_of(min_tb_size)
        {
            return Err(GeometryError::InvalidBlockSize);
        }
        Ok(Self {
            format,
            ctb_size,
            min_tb_size,
        })
    }

    /// Number of CTBs across the luma picture.
    pub const fn width_in_ctbs(self) -> u32 {
        ceil_div(self.format.width, self.ctb_size)
    }

    /// Number of CTBs down the luma picture.
    pub const fn height_in_ctbs(self) -> u32 {
        ceil_div(self.format.height, self.ctb_size)
    }

    /// Number of CTBs in the picture.
    pub const fn ctb_count(self) -> u32 {
        self.width_in_ctbs() * self.height_in_ctbs()
    }

    /// Returns the CTB raster address containing a luma sample.
    pub const fn ctb_addr_rs(self, x: u32, y: u32) -> Option<u32> {
        if x >= self.format.width || y >= self.format.height {
            return None;
        }
        Some((y / self.ctb_size) * self.width_in_ctbs() + x / self.ctb_size)
    }

    /// Returns `(x, y)` CTB coordinates for a raster address.
    pub const fn ctb_xy(self, addr_rs: u32) -> Option<(u32, u32)> {
        if addr_rs >= self.ctb_count() {
            return None;
        }
        Some((
            addr_rs % self.width_in_ctbs(),
            addr_rs / self.width_in_ctbs(),
        ))
    }

    /// Returns the valid luma rectangle covered by a CTB.
    pub fn ctb_bounds(self, addr_rs: u32) -> Option<Block> {
        let (ctb_x, ctb_y) = self.ctb_xy(addr_rs)?;
        let x = ctb_x * self.ctb_size;
        let y = ctb_y * self.ctb_size;
        Some(Block {
            x,
            y,
            width: std::cmp::min(self.format.width.saturating_sub(x), self.ctb_size),
            height: std::cmp::min(self.format.height.saturating_sub(y), self.ctb_size),
        })
    }
}

/// A rectangular luma block.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Block {
    /// Left coordinate in luma samples.
    pub x: u32,
    /// Top coordinate in luma samples.
    pub y: u32,
    /// Width in luma samples.
    pub width: u32,
    /// Height in luma samples.
    pub height: u32,
}

impl Block {
    /// Creates a square block.
    pub const fn square(x: u32, y: u32, size: u32) -> Self {
        Self {
            x,
            y,
            width: size,
            height: size,
        }
    }

    /// Returns whether a sample coordinate is inside this block.
    pub const fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }

    /// Splits an even square block into the four H.265 quadtree children.
    pub fn split(self) -> Option<[Self; 4]> {
        if self.width != self.height || self.width < 2 || !self.width.is_multiple_of(2) {
            return None;
        }
        let half = self.width / 2;
        Some([
            Self::square(self.x, self.y, half),
            Self::square(self.x + half, self.y, half),
            Self::square(self.x, self.y + half, half),
            Self::square(self.x + half, self.y + half, half),
        ])
    }
}

/// A recursively partitioned coding-tree block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuadTree {
    /// A leaf coding block.
    Leaf(Block),
    /// A split coding block and its four children in top-left, top-right,
    /// bottom-left, bottom-right order.
    Split {
        /// The block represented by this node.
        block: Block,
        /// The four child nodes.
        children: Box<[QuadTree; 4]>,
    },
}

impl QuadTree {
    /// Creates an unsplit CTB.
    pub const fn new(block: Block) -> Self {
        Self::Leaf(block)
    }

    /// Returns the block represented by this node.
    pub const fn block(&self) -> Block {
        match self {
            Self::Leaf(block) | Self::Split { block, .. } => *block,
        }
    }

    /// Splits a leaf and returns its children, or `false` when already split
    /// or when the block cannot be split evenly.
    pub fn split(&mut self) -> bool {
        let block = match self {
            Self::Leaf(block) => *block,
            Self::Split { .. } => return false,
        };
        let children = match block.split() {
            Some(children) => children,
            None => return false,
        };
        *self = Self::Split {
            block,
            children: Box::new(children.map(Self::Leaf)),
        };
        true
    }

    /// Returns all leaf coding blocks from left-to-right, top-to-bottom tree order.
    pub fn leaves(&self) -> Vec<Block> {
        let mut result = Vec::new();
        self.collect_leaves(&mut result);
        result
    }

    fn collect_leaves(&self, result: &mut Vec<Block>) {
        match self {
            Self::Leaf(block) => result.push(*block),
            Self::Split { children, .. } => {
                for child in children.iter() {
                    child.collect_leaves(result);
                }
            }
        }
    }
}

const fn ceil_div(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}

const fn is_power_of_two(value: u32) -> bool {
    value != 0 && (value & (value - 1)) == 0
}
