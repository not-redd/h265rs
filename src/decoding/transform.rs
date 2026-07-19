use super::clip_sample;
use crate::Block;
use std::sync::OnceLock;

/// Parameters used by §§8.6.2–8.6.4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransformParameters {
    /// Component index (`0 = Y`, `1 = Cb`, `2 = Cr`).
    pub component: usize,
    /// Component bit depth.
    pub bit_depth: u8,
    /// Quantization parameter.
    pub qp: i32,
    /// Whether the 4x4 intra luma transform is used.
    pub intra_4x4: bool,
    /// Whether the transform is bypassed.
    pub transform_bypass: bool,
}

/// Scales transform coefficient levels using §8.6.3's level scale table.
pub fn scale_transform_coefficients(
    levels: &[i32],
    size: usize,
    qp: i32,
    bit_depth: u8,
    scaling: Option<&[i32]>,
) -> Vec<i32> {
    assert_eq!(levels.len(), size * size);
    let level_scale = [40_i64, 45, 51, 57, 64, 72][qp.rem_euclid(6) as usize];
    let log2_size = size.trailing_zeros() as i32;
    let transform_range = i32::from(bit_depth).saturating_add(6).max(15);
    let bd_shift = i32::from(bit_depth) + log2_size + 10 - transform_range;
    let shift = bd_shift.max(0) as u32;
    let offset = if shift == 0 { 0 } else { 1_i64 << (shift - 1) };
    levels
        .iter()
        .enumerate()
        .map(|(index, &level)| {
            let matrix = scaling
                .and_then(|values| values.get(index))
                .copied()
                .unwrap_or(16) as i64;
            let value = i64::from(level) * matrix * level_scale * (1_i64 << (qp.max(0) / 6));
            clip_signed((value + offset) >> shift, transform_range)
        })
        .collect()
}

/// Performs the separable inverse integer transform from §8.6.4.
pub fn inverse_transform(
    coefficients: &[i32],
    size: usize,
    intra_4x4: bool,
    bit_depth: u8,
) -> Vec<i32> {
    assert_eq!(coefficients.len(), size * size);
    let mut intermediate = vec![0_i64; size * size];
    let mut column = vec![0_i64; size];
    let mut transformed = vec![0_i64; size];
    for x in 0..size {
        for row in 0..size {
            column[row] = i64::from(coefficients[row * size + x]);
        }
        one_dimensional_transform(&column, size, intra_4x4 && size == 4, &mut transformed);
        for row in 0..size {
            intermediate[row * size + x] = transformed[row];
        }
    }
    let mut output = vec![0; size * size];
    let mut line = vec![0_i64; size];
    for row in 0..size {
        one_dimensional_transform(
            &intermediate[row * size..(row + 1) * size],
            size,
            intra_4x4 && size == 4,
            &mut line,
        );
        for column in 0..size {
            output[row * size + column] = clip_signed(line[column], i32::from(bit_depth) + 7);
        }
    }
    output
}

fn one_dimensional_transform(input: &[i64], size: usize, intra_4x4: bool, output: &mut [i64]) {
    const MATRIX_4: [[i64; 4]; 4] = [
        [29_i64, 55, 74, 84],
        [74, 74, 0, -74],
        [84, -29, -74, 55],
        [55, -84, 74, -29],
    ];
    let matrix = (!intra_4x4).then(|| transform_matrix(size)).flatten();
    #[cfg(feature = "simd")]
    let simd_safe = input
        .iter()
        .map(|value| value.unsigned_abs())
        .max()
        .unwrap_or(0)
        .checked_mul(90)
        .and_then(|value| value.checked_mul(size as u64))
        .is_some_and(|value| value <= i32::MAX as u64);
    for row in 0..size {
        let coefficients: &[i64] = if intra_4x4 {
            &MATRIX_4[row]
        } else if let Some(matrix) = matrix {
            &matrix[row * size..(row + 1) * size]
        } else {
            output[row] = (0..size)
                .map(|column| transform_coefficient(size, row, column) * input[column])
                .sum();
            continue;
        };
        #[cfg(feature = "simd")]
        if simd_safe {
            output[row] = super::simd::dot_product_i64(input, coefficients);
            continue;
        }
        output[row] = coefficients
            .iter()
            .zip(input)
            .map(|(&coefficient, &value)| coefficient * value)
            .sum();
    }
}

fn transform_matrix(size: usize) -> Option<&'static [i64]> {
    static MATRIX_4: OnceLock<Vec<i64>> = OnceLock::new();
    static MATRIX_8: OnceLock<Vec<i64>> = OnceLock::new();
    static MATRIX_16: OnceLock<Vec<i64>> = OnceLock::new();
    static MATRIX_32: OnceLock<Vec<i64>> = OnceLock::new();
    let matrix = match size {
        4 => &MATRIX_4,
        8 => &MATRIX_8,
        16 => &MATRIX_16,
        32 => &MATRIX_32,
        _ => return None,
    };
    Some(matrix.get_or_init(|| build_transform_matrix(size)))
}

fn build_transform_matrix(size: usize) -> Vec<i64> {
    (0..size)
        .flat_map(|row| (0..size).map(move |column| transform_coefficient(size, row, column)))
        .collect()
}

fn transform_coefficient(size: usize, row: usize, column: usize) -> i64 {
    if row == 0 {
        return 64;
    }
    let value = 64.0
        * (std::f64::consts::PI * (2 * column + 1) as f64 * row as f64 / (2 * size) as f64).cos()
        * 2.0_f64.sqrt();
    value.round() as i64
}

fn clip_signed(value: i64, range: i32) -> i32 {
    let max = (1_i64 << (range - 1).clamp(1, 30)) - 1;
    value.clamp(-max - 1, max) as i32
}

/// Implements the residual modification process for transform bypass blocks.
pub fn residual_bypass(residual: &mut [i32], size: usize, horizontal: bool) {
    assert_eq!(residual.len(), size * size);
    if horizontal {
        for y in 0..size {
            for x in 1..size {
                let index = y * size + x;
                residual[index] = residual[index].saturating_add(residual[index - 1]);
            }
        }
    } else {
        for y in 1..size {
            for x in 0..size {
                let index = y * size + x;
                residual[index] = residual[index].saturating_add(residual[index - size]);
            }
        }
    }
}

/// Applies the cross-component prediction modification from §8.6.6.
pub fn cross_component_residual(
    residual: &mut [i32],
    luma_residual: &[i32],
    res_scale_val: i32,
    luma_bit_depth: u8,
    chroma_bit_depth: u8,
) {
    assert_eq!(residual.len(), luma_residual.len());
    for (chroma, &luma) in residual.iter_mut().zip(luma_residual) {
        let scaled_luma = (i64::from(luma) << chroma_bit_depth) >> luma_bit_depth;
        *chroma = chroma.saturating_add(((i64::from(res_scale_val) * scaled_luma) >> 3) as i32);
    }
}

/// Applies the adaptive colour transform residual modification in §8.6.8.2.
pub fn adaptive_colour_transform(
    luma: &mut [i32],
    cb: &mut [i32],
    cr: &mut [i32],
    bit_depth_luma: u8,
    bit_depth_chroma: u8,
    transform_bypass: bool,
) {
    assert_eq!(luma.len(), cb.len());
    assert_eq!(luma.len(), cr.len());
    let max_depth = bit_depth_luma.max(bit_depth_chroma);
    let delta_y = max_depth.saturating_sub(bit_depth_luma);
    let delta_c = max_depth.saturating_sub(bit_depth_chroma);
    let offset_y = if delta_y == 0 {
        0
    } else {
        1_i32 << (delta_y - 1)
    };
    let offset_c = if delta_c == 0 {
        0
    } else {
        1_i32 << (delta_c - 1)
    };
    for ((y, b), r) in luma.iter_mut().zip(cb.iter_mut()).zip(cr.iter_mut()) {
        if !transform_bypass {
            *y = shift_i32(*y, u32::from(delta_y));
            *b = shift_i32(*b, u32::from(delta_c + 1));
            *r = shift_i32(*r, u32::from(delta_c + 1));
        }
        let tmp = *y - (*b >> 1);
        *y = tmp + *b;
        *b = tmp - (*r >> 1);
        *r += *b;
        if !transform_bypass {
            *y = (*y + offset_y) >> delta_y;
            *b = (*b + offset_c) >> delta_c;
            *r = (*r + offset_c) >> delta_c;
        }
    }
}

fn shift_i32(value: i32, shift: u32) -> i32 {
    (i64::from(value) << shift).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Adds a prediction block and residual block, applying `Clip1Y`/`Clip1C`.
pub fn reconstruct_block(
    prediction: &[i32],
    residual: &[i32],
    block: Block,
    bit_depth: u8,
) -> Vec<i32> {
    assert_eq!(
        prediction.len(),
        block.width as usize * block.height as usize
    );
    assert_eq!(residual.len(), prediction.len());
    prediction
        .iter()
        .zip(residual)
        .map(|(&p, &r)| clip_sample(i64::from(p) + i64::from(r), bit_depth))
        .collect()
}
