//! Safe SIMD kernels used by the optional `simd` feature.

use wide::i32x4;

use super::inter::{
    fractional_chroma_sample, fractional_luma_sample, CHROMA_FILTERS, LUMA_FILTERS,
};
use super::SamplePlane;

fn load4(plane: &SamplePlane, x: i32, y: i32, max: i32) -> Option<i32x4> {
    let x = usize::try_from(x).ok()?;
    let y = usize::try_from(y).ok()?;
    let row = plane.row(y)?;
    let values = row.get(x..x.checked_add(4)?)?;
    let value = i32x4::new([values[0], values[1], values[2], values[3]]);
    (value.reduce_min() >= 0 && value.reduce_max() <= max).then_some(value)
}

fn store4(output: &mut [i32], index: usize, value: i32x4) {
    output[index..index + 4].copy_from_slice(&value.to_array());
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_luma_prediction(
    plane: &SamplePlane,
    start_x: i32,
    start_y: i32,
    width: usize,
    height: usize,
    frac_x: u8,
    frac_y: u8,
    bit_depth: u8,
    output: &mut [i32],
) {
    let max = ((1_i64 << bit_depth) - 1) as i32;
    let minimum = i32x4::ZERO;
    let maximum = i32x4::splat(max);
    let horizontal = LUMA_FILTERS[frac_x as usize];
    let vertical = LUMA_FILTERS[frac_y as usize];
    for row in 0..height {
        let y = start_y + row as i32;
        let mut column = 0;
        while column + 4 <= width {
            let x = start_x + column as i32;
            let value: Option<i32x4> = if frac_y == 0 {
                filter_luma_horizontal(plane, x, y, &horizontal, max).map(|value| value >> 6)
            } else if frac_x == 0 {
                filter_luma_vertical(plane, x, y, &vertical, max).map(|value| value >> 6)
            } else {
                let mut sum = i32x4::ZERO;
                let mut valid = true;
                for (tap, &coefficient) in vertical.iter().enumerate() {
                    let Some(horizontal_value) =
                        filter_luma_horizontal(plane, x, y + tap as i32 - 3, &horizontal, max)
                    else {
                        valid = false;
                        break;
                    };
                    sum += (horizontal_value >> 6) * i32x4::splat(coefficient);
                }
                valid.then_some((sum + i32x4::splat(2048)) >> 12)
            };
            if let Some(value) = value {
                store4(
                    output,
                    row * width + column,
                    value.max(minimum).min(maximum),
                );
            } else {
                for lane in 0..4 {
                    output[row * width + column + lane] = fractional_luma_sample(
                        plane,
                        x + lane as i32,
                        y,
                        frac_x,
                        frac_y,
                        bit_depth,
                    );
                }
            }
            column += 4;
        }
        for x in column..width {
            output[row * width + x] =
                fractional_luma_sample(plane, start_x + x as i32, y, frac_x, frac_y, bit_depth);
        }
    }
}

fn filter_luma_horizontal(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    coefficients: &[i32; 8],
    max: i32,
) -> Option<i32x4> {
    let mut sum = i32x4::ZERO;
    for (tap, &coefficient) in coefficients.iter().enumerate() {
        sum += load4(plane, x + tap as i32 - 3, y, max)? * i32x4::splat(coefficient);
    }
    Some(sum)
}

fn filter_luma_vertical(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    coefficients: &[i32; 8],
    max: i32,
) -> Option<i32x4> {
    let mut sum = i32x4::ZERO;
    for (tap, &coefficient) in coefficients.iter().enumerate() {
        sum += load4(plane, x, y + tap as i32 - 3, max)? * i32x4::splat(coefficient);
    }
    Some(sum)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_chroma_prediction(
    plane: &SamplePlane,
    start_x: i32,
    start_y: i32,
    width: usize,
    height: usize,
    frac_x: u8,
    frac_y: u8,
    bit_depth: u8,
    output: &mut [i32],
) {
    let max = ((1_i64 << bit_depth) - 1) as i32;
    let minimum = i32x4::ZERO;
    let maximum = i32x4::splat(max);
    let horizontal = CHROMA_FILTERS[frac_x as usize];
    let vertical = CHROMA_FILTERS[frac_y as usize];
    for row in 0..height {
        let y = start_y + row as i32;
        let mut column = 0;
        while column + 4 <= width {
            let x = start_x + column as i32;
            let value: Option<i32x4> = if frac_y == 0 {
                filter_chroma_horizontal(plane, x, y, &horizontal, max).map(|value| value >> 6)
            } else if frac_x == 0 {
                filter_chroma_vertical(plane, x, y, &vertical, max).map(|value| value >> 6)
            } else {
                let mut sum = i32x4::ZERO;
                let mut valid = true;
                for (tap, &coefficient) in vertical.iter().enumerate() {
                    let Some(horizontal_value) =
                        filter_chroma_horizontal(plane, x, y + tap as i32 - 1, &horizontal, max)
                    else {
                        valid = false;
                        break;
                    };
                    sum += (horizontal_value >> 6) * i32x4::splat(coefficient);
                }
                valid.then_some(sum >> 6)
            };
            if let Some(value) = value {
                store4(
                    output,
                    row * width + column,
                    value.max(minimum).min(maximum),
                );
            } else {
                for lane in 0..4 {
                    output[row * width + column + lane] = fractional_chroma_sample(
                        plane,
                        x + lane as i32,
                        y,
                        frac_x,
                        frac_y,
                        bit_depth,
                    );
                }
            }
            column += 4;
        }
        for x in column..width {
            output[row * width + x] =
                fractional_chroma_sample(plane, start_x + x as i32, y, frac_x, frac_y, bit_depth);
        }
    }
}

fn filter_chroma_horizontal(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    coefficients: &[i32; 4],
    max: i32,
) -> Option<i32x4> {
    let mut sum = i32x4::ZERO;
    for (tap, &coefficient) in coefficients.iter().enumerate() {
        sum += load4(plane, x + tap as i32 - 1, y, max)? * i32x4::splat(coefficient);
    }
    Some(sum)
}

fn filter_chroma_vertical(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    coefficients: &[i32; 4],
    max: i32,
) -> Option<i32x4> {
    let mut sum = i32x4::ZERO;
    for (tap, &coefficient) in coefficients.iter().enumerate() {
        sum += load4(plane, x, y + tap as i32 - 1, max)? * i32x4::splat(coefficient);
    }
    Some(sum)
}

pub(super) fn dot_product_i64(input: &[i64], coefficients: &[i64]) -> i64 {
    debug_assert_eq!(input.len(), coefficients.len());
    let mut accumulator = i32x4::ZERO;
    let mut input_chunks = input.chunks_exact(4);
    let mut coefficient_chunks = coefficients.chunks_exact(4);
    for (values, factors) in input_chunks.by_ref().zip(coefficient_chunks.by_ref()) {
        accumulator += i32x4::new([
            values[0] as i32,
            values[1] as i32,
            values[2] as i32,
            values[3] as i32,
        ]) * i32x4::new([
            factors[0] as i32,
            factors[1] as i32,
            factors[2] as i32,
            factors[3] as i32,
        ]);
    }
    let tail = input_chunks
        .remainder()
        .iter()
        .zip(coefficient_chunks.remainder())
        .map(|(&value, &factor)| value * factor)
        .sum::<i64>();
    i64::from(accumulator.reduce_add()) + tail
}

#[allow(clippy::too_many_arguments)]
pub(super) fn default_weighted_prediction(
    l0: &[i32],
    l1: &[i32],
    has0: bool,
    has1: bool,
    shift1: u32,
    shift2: u32,
    offset1: i32,
    offset2: i32,
    max: i32,
) -> Vec<i32> {
    let mut output = Vec::with_capacity(l0.len());
    let mut l0_chunks = l0.chunks_exact(4);
    let mut l1_chunks = l1.chunks_exact(4);
    for (a, b) in l0_chunks.by_ref().zip(l1_chunks.by_ref()) {
        let a = i32x4::new([a[0], a[1], a[2], a[3]]);
        let b = i32x4::new([b[0], b[1], b[2], b[3]]);
        let value = match (has0, has1) {
            (true, false) => a.saturating_add(i32x4::splat(offset1)) >> shift1,
            (false, true) => b.saturating_add(i32x4::splat(offset1)) >> shift1,
            (true, true) => a.saturating_add(b).saturating_add(i32x4::splat(offset2)) >> shift2,
            (false, false) => i32x4::ZERO,
        };
        output.extend_from_slice(&value.max(i32x4::ZERO).min(i32x4::splat(max)).to_array());
    }
    output.extend(
        l0_chunks
            .remainder()
            .iter()
            .zip(l1_chunks.remainder())
            .map(|(&a, &b)| {
                let value = match (has0, has1) {
                    (true, false) => (i64::from(a) + i64::from(offset1)) >> shift1,
                    (false, true) => (i64::from(b) + i64::from(offset1)) >> shift1,
                    (true, true) => (i64::from(a) + i64::from(b) + i64::from(offset2)) >> shift2,
                    (false, false) => 0,
                };
                value.clamp(0, i64::from(max)) as i32
            }),
    );
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar_luma_block(
        plane: &SamplePlane,
        start_x: i32,
        start_y: i32,
        width: usize,
        height: usize,
        frac_x: u8,
        frac_y: u8,
    ) -> Vec<i32> {
        let mut output = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                output.push(fractional_luma_sample(
                    plane,
                    start_x + x as i32,
                    start_y + y as i32,
                    frac_x,
                    frac_y,
                    8,
                ));
            }
        }
        output
    }

    fn scalar_chroma_block(
        plane: &SamplePlane,
        start_x: i32,
        start_y: i32,
        width: usize,
        height: usize,
        frac_x: u8,
        frac_y: u8,
    ) -> Vec<i32> {
        let mut output = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                output.push(fractional_chroma_sample(
                    plane,
                    start_x + x as i32,
                    start_y + y as i32,
                    frac_x,
                    frac_y,
                    8,
                ));
            }
        }
        output
    }

    #[test]
    fn weighted_prediction_matches_widened_scalar_math() {
        let values = [i32::MIN, -65_536, -1, 0, 1, 255, 65_535, i32::MAX];
        for bit_depth in [8_u8, 10, 16] {
            let shift1 = u32::from((14_i32 - i32::from(bit_depth)).max(2) as u8);
            let shift2 = u32::from((15_i32 - i32::from(bit_depth)).max(3) as u8);
            let offset1 = 1_i32 << shift1.saturating_sub(1);
            let offset2 = 1_i32 << shift2.saturating_sub(1);
            let max = ((1_i64 << bit_depth) - 1) as i32;
            for flags in [(false, false), (true, false), (false, true), (true, true)] {
                let actual = default_weighted_prediction(
                    &values,
                    &values.iter().rev().copied().collect::<Vec<_>>(),
                    flags.0,
                    flags.1,
                    shift1,
                    shift2,
                    offset1,
                    offset2,
                    max,
                );
                let expected: Vec<_> = values
                    .iter()
                    .zip(values.iter().rev())
                    .map(|(&a, &b)| {
                        let value = match flags {
                            (true, false) => (i64::from(a) + i64::from(offset1)) >> shift1,
                            (false, true) => (i64::from(b) + i64::from(offset1)) >> shift1,
                            (true, true) => {
                                (i64::from(a) + i64::from(b) + i64::from(offset2)) >> shift2
                            }
                            (false, false) => 0,
                        };
                        value.clamp(0, i64::from(max)) as i32
                    })
                    .collect();
                assert_eq!(actual, expected, "bit depth {bit_depth}, flags {flags:?}");
            }
        }
    }

    #[test]
    fn transform_dot_product_matches_scalar_math() {
        let input: Vec<i64> = (-16..16).map(|value| i64::from(value * 127)).collect();
        let coefficients: Vec<i64> = (0..32)
            .map(|index| i64::from((index * 37) % 181 - 90))
            .collect();
        let expected = input
            .iter()
            .zip(&coefficients)
            .map(|(&value, &coefficient)| value * coefficient)
            .sum();
        assert_eq!(dot_product_i64(&input, &coefficients), expected);
    }

    #[test]
    fn block_interpolation_matches_scalar_samples() {
        let samples: Vec<_> = (0..24 * 24)
            .map(|index| (index * 29 + index / 7) & 255)
            .collect();
        let plane = SamplePlane::from_samples(24, 24, samples).unwrap();
        for frac_y in 0..4 {
            for frac_x in 0..4 {
                let mut actual = vec![0; 12 * 8];
                fill_luma_prediction(&plane, 5, 5, 12, 8, frac_x, frac_y, 8, &mut actual);
                let expected = scalar_luma_block(&plane, 5, 5, 12, 8, frac_x, frac_y);
                assert_eq!(actual, expected, "luma fractions ({frac_x}, {frac_y})");
            }
        }
        for frac_y in 0..8 {
            for frac_x in 0..8 {
                let mut actual = vec![0; 12 * 8];
                fill_chroma_prediction(&plane, 3, 3, 12, 8, frac_x, frac_y, 8, &mut actual);
                let expected = scalar_chroma_block(&plane, 3, 3, 12, 8, frac_x, frac_y);
                assert_eq!(actual, expected, "chroma fractions ({frac_x}, {frac_y})");
            }
        }
    }

    #[test]
    fn block_interpolation_uses_scalar_edges() {
        let plane =
            SamplePlane::from_samples(5, 3, (0..15).map(|value| value * 7).collect()).unwrap();
        let mut actual = vec![0; 5 * 3];
        fill_luma_prediction(&plane, -1, -1, 5, 3, 2, 3, 8, &mut actual);
        let expected = scalar_luma_block(&plane, -1, -1, 5, 3, 2, 3);
        assert_eq!(actual, expected);
    }
}
