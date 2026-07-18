use crate::{min_tb_address_z_scan, Block, GeometryError, PictureGeometry, TileLayout};

/// A prediction mode used by §6.4.2's availability process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredictionMode {
    /// Intra prediction.
    Intra,
    /// Inter prediction.
    Inter,
}

/// Slice/tile information needed to answer neighbour-availability queries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvailabilityContext {
    /// Picture geometry.
    pub geometry: PictureGeometry,
    /// Tile layout for the picture.
    pub tiles: TileLayout,
    /// Slice-segment raster address for each CTU.
    pub slice_segment_of_ctu: Vec<u32>,
}

impl AvailabilityContext {
    /// Creates an availability context. A slice-segment entry is required for
    /// every CTU in raster order.
    pub fn new(
        geometry: PictureGeometry,
        tiles: TileLayout,
        slice_segment_of_ctu: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        if tiles.dimensions() != (geometry.width_in_ctbs(), geometry.height_in_ctbs())
            || slice_segment_of_ctu.len() != geometry.ctb_count() as usize
        {
            return Err(GeometryError::TileDimensionsDoNotCoverPicture);
        }
        Ok(Self {
            geometry,
            tiles,
            slice_segment_of_ctu,
        })
    }

    fn block_min_tb_address(&self, x: u32, y: u32) -> Option<u32> {
        if x >= self.geometry.format.width || y >= self.geometry.format.height {
            return None;
        }
        Some(min_tb_address_z_scan(&self.geometry, x, y))
    }

    /// Implements §6.4.1 for a current block and candidate neighbour.
    pub fn z_scan_block_available(&self, current: (u32, u32), neighbour: (u32, u32)) -> bool {
        let current_addr = match self.block_min_tb_address(current.0, current.1) {
            Some(addr) => addr,
            None => return false,
        };
        let neighbour_addr = match self.block_min_tb_address(neighbour.0, neighbour.1) {
            Some(addr) => addr,
            None => return false,
        };
        if neighbour_addr > current_addr {
            return false;
        }
        let current_ctu = match self.geometry.ctb_addr_rs(current.0, current.1) {
            Some(addr) => addr,
            None => return false,
        };
        let neighbour_ctu = match self.geometry.ctb_addr_rs(neighbour.0, neighbour.1) {
            Some(addr) => addr,
            None => return false,
        };
        self.slice_segment_of_ctu[current_ctu as usize]
            == self.slice_segment_of_ctu[neighbour_ctu as usize]
            && self.tiles.tile_id(current_ctu) == self.tiles.tile_id(neighbour_ctu)
    }

    /// Implements §6.4.2 for a candidate neighbouring prediction block.
    pub fn prediction_block_available(
        &self,
        coding_block: Block,
        prediction_block: Block,
        part_idx: u32,
        neighbour: (u32, u32),
        neighbour_mode: PredictionMode,
    ) -> bool {
        let same_cb = coding_block.contains(neighbour.0, neighbour.1);
        let mut available = if !same_cb {
            self.z_scan_block_available((prediction_block.x, prediction_block.y), neighbour)
        } else {
            let unavailable_same_partition = prediction_block.width * 2 == coding_block.width
                && prediction_block.height * 2 == coding_block.height
                && part_idx == 1
                && coding_block.y + prediction_block.height <= neighbour.1
                && coding_block.x + prediction_block.width > neighbour.0;
            !unavailable_same_partition
        };
        if available && neighbour_mode == PredictionMode::Intra {
            available = false;
        }
        available
    }
}
