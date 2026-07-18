use crate::{GeometryError, PictureGeometry};

/// Returns the minimum transform-block address in Z-scan order for a luma
/// sample coordinate.
pub fn min_tb_address_z_scan(geometry: &PictureGeometry, x: u32, y: u32) -> u32 {
    let min_per_ctb = geometry.ctb_size / geometry.min_tb_size;
    let ctb_x = x / geometry.ctb_size;
    let ctb_y = y / geometry.ctb_size;
    let local_x = (x % geometry.ctb_size) / geometry.min_tb_size;
    let local_y = (y % geometry.ctb_size) / geometry.min_tb_size;
    let ctb_addr = ctb_y * geometry.width_in_ctbs() + ctb_x;
    (ctb_addr * min_per_ctb * min_per_ctb) + z_scan_rank(min_per_ctb, local_x, local_y)
}

/// Returns the `(x, y)` coordinates of a square block in Z-scan order.
pub fn z_scan_order(size: u32) -> Result<Vec<(u32, u32)>, GeometryError> {
    validate_scan_size(size)?;
    Ok((0..size * size)
        .map(|rank| z_scan_coordinate(size, rank))
        .collect())
}

/// Returns an address table equivalent to `MinTbAddrZs[x][y]` in §6.5.2.
pub fn min_tb_address_table(geometry: &PictureGeometry) -> Vec<Vec<u32>> {
    let width = geometry.width_in_ctbs() * geometry.ctb_size / geometry.min_tb_size;
    let height = geometry.height_in_ctbs() * geometry.ctb_size / geometry.min_tb_size;
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    min_tb_address_z_scan(
                        geometry,
                        x * geometry.min_tb_size,
                        y * geometry.min_tb_size,
                    )
                })
                .collect()
        })
        .collect()
}

/// Returns an up-right diagonal scan for a square block.
pub fn up_right_diagonal_scan(size: u32) -> Result<Vec<(u32, u32)>, GeometryError> {
    validate_scan_size(size)?;
    let mut result = Vec::with_capacity((size * size) as usize);
    let mut x = 0i64;
    let mut y = 0i64;
    while result.len() < (size * size) as usize {
        while y >= 0 {
            if x < i64::from(size) && y < i64::from(size) {
                result.push((x as u32, y as u32));
            }
            y -= 1;
            x += 1;
        }
        y = x;
        x = 0;
    }
    Ok(result)
}

/// Returns a row-major horizontal scan for a square block.
pub fn horizontal_scan(size: u32) -> Result<Vec<(u32, u32)>, GeometryError> {
    validate_scan_size(size)?;
    Ok((0..size)
        .flat_map(|y| (0..size).map(move |x| (x, y)))
        .collect())
}

/// Returns a column-major vertical scan for a square block.
pub fn vertical_scan(size: u32) -> Result<Vec<(u32, u32)>, GeometryError> {
    validate_scan_size(size)?;
    Ok((0..size)
        .flat_map(|x| (0..size).map(move |y| (x, y)))
        .collect())
}

/// Returns the alternating-row traverse scan for a square block.
pub fn traverse_scan(size: u32) -> Result<Vec<(u32, u32)>, GeometryError> {
    validate_scan_size(size)?;
    let mut result = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        if y % 2 == 0 {
            result.extend((0..size).map(|x| (x, y)));
        } else {
            result.extend((0..size).rev().map(|x| (x, y)));
        }
    }
    Ok(result)
}

fn validate_scan_size(size: u32) -> Result<(), GeometryError> {
    if !is_power_of_two(size) {
        Err(GeometryError::InvalidBlockSize)
    } else {
        Ok(())
    }
}

const fn is_power_of_two(value: u32) -> bool {
    value != 0 && (value & (value - 1)) == 0
}

fn z_scan_rank(size: u32, x: u32, y: u32) -> u32 {
    let mut rank = 0;
    let mut bit = 0;
    while (1u32 << bit) < size {
        rank |= ((x >> bit) & 1) << (2 * bit);
        rank |= ((y >> bit) & 1) << (2 * bit + 1);
        bit += 1;
    }
    rank
}

fn z_scan_coordinate(size: u32, rank: u32) -> (u32, u32) {
    let mut x = 0;
    let mut y = 0;
    let mut bit = 0;
    while (1u32 << bit) < size {
        x |= ((rank >> (2 * bit)) & 1) << bit;
        y |= ((rank >> (2 * bit + 1)) & 1) << bit;
        bit += 1;
    }
    (x, y)
}
