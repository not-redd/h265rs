use crate::GeometryError;

/// Tile dimensions and CTB-address conversions from §6.5.1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TileLayout {
    picture_width_in_ctbs: u32,
    picture_height_in_ctbs: u32,
    column_widths: Vec<u32>,
    row_heights: Vec<u32>,
    raster_to_tile: Vec<u32>,
    tile_to_raster: Vec<u32>,
    tile_ids: Vec<u32>,
}

impl TileLayout {
    /// Creates uniformly spaced tile columns and rows.
    pub fn uniform(
        picture_width_in_ctbs: u32,
        picture_height_in_ctbs: u32,
        columns: u32,
        rows: u32,
    ) -> Result<Self, GeometryError> {
        if picture_width_in_ctbs == 0
            || picture_height_in_ctbs == 0
            || columns == 0
            || rows == 0
            || columns > picture_width_in_ctbs
            || rows > picture_height_in_ctbs
        {
            return Err(GeometryError::TileDimensionsDoNotCoverPicture);
        }
        let column_widths = split_uniform(picture_width_in_ctbs, columns);
        let row_heights = split_uniform(picture_height_in_ctbs, rows);
        Self::from_dimensions(
            picture_width_in_ctbs,
            picture_height_in_ctbs,
            column_widths,
            row_heights,
        )
    }

    /// Creates a tile layout using explicit CTB widths and heights.
    pub fn explicit(
        picture_width_in_ctbs: u32,
        picture_height_in_ctbs: u32,
        column_widths: Vec<u32>,
        row_heights: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        Self::from_dimensions(
            picture_width_in_ctbs,
            picture_height_in_ctbs,
            column_widths,
            row_heights,
        )
    }

    fn from_dimensions(
        picture_width_in_ctbs: u32,
        picture_height_in_ctbs: u32,
        column_widths: Vec<u32>,
        row_heights: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        if picture_width_in_ctbs == 0
            || picture_height_in_ctbs == 0
            || column_widths.is_empty()
            || row_heights.is_empty()
            || column_widths.contains(&0)
            || row_heights.contains(&0)
            || column_widths.iter().sum::<u32>() != picture_width_in_ctbs
            || row_heights.iter().sum::<u32>() != picture_height_in_ctbs
        {
            return Err(if column_widths.contains(&0) || row_heights.contains(&0) {
                GeometryError::ZeroSizedTile
            } else {
                GeometryError::TileDimensionsDoNotCoverPicture
            });
        }

        let ctb_count = picture_width_in_ctbs * picture_height_in_ctbs;
        let mut raster_to_tile = vec![0; ctb_count as usize];
        let mut tile_to_raster = Vec::with_capacity(ctb_count as usize);
        let mut tile_ids = vec![0; ctb_count as usize];
        let column_bounds = cumulative_bounds(&column_widths);
        let row_bounds = cumulative_bounds(&row_heights);
        let mut tile_addr = 0u32;
        for tile_row in 0..row_heights.len() as u32 {
            for tile_col in 0..column_widths.len() as u32 {
                let tile_id = tile_row * column_widths.len() as u32 + tile_col;
                let x0 = column_bounds[tile_col as usize];
                let y0 = row_bounds[tile_row as usize];
                let x1 = column_bounds[tile_col as usize + 1];
                let y1 = row_bounds[tile_row as usize + 1];
                for y in y0..y1 {
                    for x in x0..x1 {
                        let raster_addr = y * picture_width_in_ctbs + x;
                        raster_to_tile[raster_addr as usize] = tile_addr;
                        tile_to_raster.push(raster_addr);
                        tile_ids[raster_addr as usize] = tile_id;
                        tile_addr += 1;
                    }
                }
            }
        }
        Ok(Self {
            picture_width_in_ctbs,
            picture_height_in_ctbs,
            column_widths,
            row_heights,
            raster_to_tile,
            tile_to_raster,
            tile_ids,
        })
    }

    /// Number of tile columns.
    pub const fn tile_columns(&self) -> usize {
        self.column_widths.len()
    }

    /// Number of tile rows.
    pub const fn tile_rows(&self) -> usize {
        self.row_heights.len()
    }

    /// Tile-column widths in CTBs.
    pub fn column_widths(&self) -> &[u32] {
        &self.column_widths
    }

    /// Tile-row heights in CTBs.
    pub fn row_heights(&self) -> &[u32] {
        &self.row_heights
    }

    /// Converts a CTB raster address to tile-scan address.
    pub fn raster_to_tile_scan(&self, addr_rs: u32) -> Option<u32> {
        self.raster_to_tile.get(addr_rs as usize).copied()
    }

    /// Converts a CTB tile-scan address to raster address.
    pub fn tile_scan_to_raster(&self, addr_ts: u32) -> Option<u32> {
        self.tile_to_raster.get(addr_ts as usize).copied()
    }

    /// Returns the tile ID containing a raster-addressed CTB.
    pub fn tile_id(&self, addr_rs: u32) -> Option<u32> {
        self.tile_ids.get(addr_rs as usize).copied()
    }

    /// Returns the CTB-grid dimensions.
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.picture_width_in_ctbs, self.picture_height_in_ctbs)
    }
}

fn split_uniform(total: u32, parts: u32) -> Vec<u32> {
    (0..parts)
        .map(|index| ((index + 1) * total) / parts - (index * total) / parts)
        .collect()
}

fn cumulative_bounds(lengths: &[u32]) -> Vec<u32> {
    let mut bounds = Vec::with_capacity(lengths.len() + 1);
    bounds.push(0);
    for &length in lengths {
        bounds.push(bounds.last().copied().unwrap_or(0) + length);
    }
    bounds
}
