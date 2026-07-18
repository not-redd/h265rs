use super::{clip_sample, DecodedPicture, SamplePlane};
use crate::{Block, ChromaFormat};

/// Motion vector in quarter-luma-sample units.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MotionVector {
    /// Horizontal component.
    pub x: i32,
    /// Vertical component.
    pub y: i32,
}

/// A motion-vector predictor and its associated reference index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionVectorPrediction {
    /// Reference index.
    pub reference_index: usize,
    /// Motion vector.
    pub motion_vector: MotionVector,
}

/// Reference-list usage for one prediction block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredictionLists {
    /// L0 reference, when present.
    pub list0: Option<MotionVectorPrediction>,
    /// L1 reference, when present.
    pub list1: Option<MotionVectorPrediction>,
}

/// Explicit weighted-prediction parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WeightParameters {
    /// Weight denominator.
    pub log2_denom: u8,
    /// Luma/chroma weight.
    pub weight: i32,
    /// Prediction offset.
    pub offset: i32,
}

/// Derives a luma motion vector from an MVP and MVD (§8.5.3.2.1).
pub fn derive_motion_vector(
    predictor: MotionVector,
    difference: MotionVector,
    reference_is_current: bool,
    use_integer_mv: bool,
) -> MotionVector {
    fn wrap(value: i64) -> i32 {
        let unsigned = value.rem_euclid(65_536);
        if unsigned >= 32_768 {
            (unsigned - 65_536) as i32
        } else {
            unsigned as i32
        }
    }
    if reference_is_current || use_integer_mv {
        MotionVector {
            x: wrap(((predictor.x >> 2) as i64 + i64::from(difference.x)) << 2),
            y: wrap(((predictor.y >> 2) as i64 + i64::from(difference.y)) << 2),
        }
    } else {
        MotionVector {
            x: wrap(i64::from(predictor.x) + i64::from(difference.x)),
            y: wrap(i64::from(predictor.y) + i64::from(difference.y)),
        }
    }
}

/// Converts a quarter-luma motion vector to eighth-chroma units (§8.5.3.2.10).
pub const fn derive_chroma_motion_vector(
    motion: MotionVector,
    subsampling: (u32, u32),
) -> MotionVector {
    MotionVector {
        x: motion.x * 2 / subsampling.0 as i32,
        y: motion.y * 2 / subsampling.1 as i32,
    }
}

/// Derives a merge candidate list from spatial candidates and fills it with
/// the combined/zero candidates required by §§8.5.3.2.3–8.5.3.2.5.
pub fn derive_merge_candidates(
    spatial: &[PredictionLists],
    max_candidates: usize,
    list1_enabled: bool,
) -> Vec<PredictionLists> {
    let mut result = Vec::new();
    for candidate in spatial {
        if result.len() == max_candidates {
            break;
        }
        if !result.contains(candidate) {
            result.push(*candidate);
        }
    }
    let original_count = result.len();
    if list1_enabled {
        for left in spatial.iter().take(original_count) {
            for right in spatial
                .iter()
                .skip(1)
                .take(original_count.saturating_sub(1))
            {
                if result.len() == max_candidates {
                    break;
                }
                let Some(list0) = left.list0 else { continue };
                let Some(list1) = right.list1 else { continue };
                let candidate = PredictionLists {
                    list0: Some(list0),
                    list1: Some(list1),
                };
                if !result.contains(&candidate) {
                    result.push(candidate);
                }
            }
        }
    }
    let reference_count = max_candidates.max(1);
    let mut zero_index = 0;
    while result.len() < max_candidates {
        let candidate = if list1_enabled {
            PredictionLists {
                list0: Some(MotionVectorPrediction {
                    reference_index: zero_index % reference_count,
                    motion_vector: MotionVector::default(),
                }),
                list1: Some(MotionVectorPrediction {
                    reference_index: zero_index % reference_count,
                    motion_vector: MotionVector::default(),
                }),
            }
        } else {
            PredictionLists {
                list0: Some(MotionVectorPrediction {
                    reference_index: zero_index % reference_count,
                    motion_vector: MotionVector::default(),
                }),
                list1: None,
            }
        };
        if !result.contains(&candidate) {
            result.push(candidate);
        } else {
            zero_index += 1;
        }
    }
    result
}

/// Generates an inter-predicted block from one or two reference pictures.
pub fn inter_predict(
    references: &[&DecodedPicture],
    block: Block,
    lists: PredictionLists,
    format: ChromaFormat,
    component: usize,
    bit_depth: u8,
) -> Vec<i32> {
    let (sub_width, sub_height) = format.subsampling();
    let scale_x = if component == 0 { 1 } else { sub_width };
    let scale_y = if component == 0 { 1 } else { sub_height };
    let width = (block.width / scale_x.max(1)) as usize;
    let height = (block.height / scale_y.max(1)) as usize;
    let mut l0 = vec![0; width * height];
    let mut l1 = vec![0; width * height];
    let mut has0 = false;
    let mut has1 = false;
    if let Some(prediction) = lists.list0 {
        if let Some(reference) = references.get(prediction.reference_index) {
            if let Some(plane) = reference.plane(component) {
                has0 = true;
                fill_prediction(
                    plane,
                    block,
                    prediction.motion_vector,
                    scale_x,
                    scale_y,
                    bit_depth,
                    &mut l0,
                );
            }
        }
    }
    if let Some(prediction) = lists.list1 {
        if let Some(reference) = references.get(prediction.reference_index) {
            if let Some(plane) = reference.plane(component) {
                has1 = true;
                fill_prediction(
                    plane,
                    block,
                    prediction.motion_vector,
                    scale_x,
                    scale_y,
                    bit_depth,
                    &mut l1,
                );
            }
        }
    }
    default_weighted_prediction(&l0, &l1, has0, has1, bit_depth)
}

fn fill_prediction(
    plane: &SamplePlane,
    block: Block,
    mv: MotionVector,
    scale_x: u32,
    scale_y: u32,
    bit_depth: u8,
    output: &mut [i32],
) {
    let width = (block.width / scale_x.max(1)) as usize;
    let height = (block.height / scale_y.max(1)) as usize;
    for y in 0..height {
        for x in 0..width {
            let x_luma = block.x / scale_x + x as u32;
            let y_luma = block.y / scale_y + y as u32;
            let (fraction_bits, mv_x, mv_y) = if scale_x == 1 {
                (2, mv.x, mv.y)
            } else {
                (3, mv.x * 2 / scale_x as i32, mv.y * 2 / scale_y as i32)
            };
            let x_int = x_luma as i32 + (mv_x >> fraction_bits);
            let y_int = y_luma as i32 + (mv_y >> fraction_bits);
            let x_frac = (mv_x & ((1 << fraction_bits) - 1)) as u8;
            let y_frac = (mv_y & ((1 << fraction_bits) - 1)) as u8;
            output[y * width + x] = if fraction_bits == 2 {
                fractional_luma_sample(plane, x_int, y_int, x_frac, y_frac, bit_depth)
            } else {
                fractional_chroma_sample(plane, x_int, y_int, x_frac, y_frac, bit_depth)
            };
        }
    }
}

/// Applies §8.5.3.3.4.2 default weighted prediction.
pub fn default_weighted_prediction(
    l0: &[i32],
    l1: &[i32],
    has0: bool,
    has1: bool,
    bit_depth: u8,
) -> Vec<i32> {
    assert_eq!(l0.len(), l1.len());
    let shift1 = u32::from((14_i32 - i32::from(bit_depth)).max(2) as u8);
    let shift2 = u32::from((15_i32 - i32::from(bit_depth)).max(3) as u8);
    let offset1 = 1_i64 << shift1.saturating_sub(1);
    let offset2 = 1_i64 << shift2.saturating_sub(1);
    l0.iter()
        .zip(l1)
        .map(|(&a, &b)| {
            let value = match (has0, has1) {
                (true, false) => (i64::from(a) + offset1) >> shift1,
                (false, true) => (i64::from(b) + offset1) >> shift1,
                (true, true) => (i64::from(a) + i64::from(b) + offset2) >> shift2,
                (false, false) => 0,
            };
            clip_sample(value, bit_depth)
        })
        .collect()
}

/// Applies explicit weighted prediction for one pair of blocks.
pub fn weighted_prediction(
    l0: &[i32],
    l1: &[i32],
    flags: (bool, bool),
    weights: (WeightParameters, WeightParameters),
    bit_depth: u8,
) -> Vec<i32> {
    assert_eq!(l0.len(), l1.len());
    let (w0, w1) = weights;
    let shift = u32::from(w0.log2_denom.max(w1.log2_denom));
    let round = if shift == 0 { 0 } else { 1_i64 << (shift - 1) };
    l0.iter()
        .zip(l1)
        .map(|(&a, &b)| {
            let value = match flags {
                (true, false) => {
                    ((i64::from(a) * i64::from(w0.weight) + round) >> shift) + i64::from(w0.offset)
                }
                (false, true) => {
                    ((i64::from(b) * i64::from(w1.weight) + round) >> shift) + i64::from(w1.offset)
                }
                (true, true) => {
                    ((i64::from(a) * i64::from(w0.weight)
                        + i64::from(b) * i64::from(w1.weight)
                        + round)
                        >> (shift + 1))
                        + ((i64::from(w0.offset) + i64::from(w1.offset) + 1) >> 1)
                }
                (false, false) => 0,
            };
            clip_sample(value, bit_depth)
        })
        .collect()
}

/// Fractional luma interpolation from Table 8-8 and the 8-tap filters in §8.5.3.3.3.2.
pub fn fractional_luma_sample(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    frac_x: u8,
    frac_y: u8,
    bit_depth: u8,
) -> i32 {
    if frac_x == 0 && frac_y == 0 {
        return clip_sample(i64::from(plane.get_clipped(x, y)), bit_depth);
    }
    let horizontal = [
        [0, 0, 0, 64, 0, 0, 0, 0],
        [-1, 4, -10, 58, 17, -5, 1, 0],
        [-1, 4, -11, 40, 40, -11, 4, -1],
        [0, 1, -5, 17, 58, -10, 4, -1],
    ];
    let vertical = horizontal;
    let sample = if frac_y == 0 {
        filter_1d(plane, x, y, frac_x, &horizontal[frac_x as usize])
    } else if frac_x == 0 {
        filter_1d_vertical(plane, x, y, frac_y, &vertical[frac_y as usize])
    } else {
        let mut intermediate = [0_i64; 8];
        for (index, item) in intermediate.iter_mut().enumerate() {
            *item = filter_1d(
                plane,
                x,
                y + index as i32 - 3,
                frac_x,
                &horizontal[frac_x as usize],
            );
        }
        let sum: i64 = vertical[frac_y as usize]
            .iter()
            .enumerate()
            .map(|(index, &coefficient)| i64::from(coefficient) * intermediate[index])
            .sum();
        (sum + 2048) >> 12
    };
    clip_sample(sample, bit_depth)
}

fn filter_1d(plane: &SamplePlane, x: i32, y: i32, _fraction: u8, coefficients: &[i32; 8]) -> i64 {
    coefficients
        .iter()
        .enumerate()
        .map(|(index, &coefficient)| {
            i64::from(coefficient) * i64::from(plane.get_clipped(x + index as i32 - 3, y))
        })
        .sum::<i64>()
        >> 6
}

fn filter_1d_vertical(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    _fraction: u8,
    coefficients: &[i32; 8],
) -> i64 {
    coefficients
        .iter()
        .enumerate()
        .map(|(index, &coefficient)| {
            i64::from(coefficient) * i64::from(plane.get_clipped(x, y + index as i32 - 3))
        })
        .sum::<i64>()
        >> 6
}

/// Fractional chroma interpolation using the seven 4-tap filters in §8.5.3.3.3.3.
pub fn fractional_chroma_sample(
    plane: &SamplePlane,
    x: i32,
    y: i32,
    frac_x: u8,
    frac_y: u8,
    bit_depth: u8,
) -> i32 {
    const FILTERS: [[i32; 4]; 8] = [
        [0, 64, 0, 0],
        [-2, 58, 10, -2],
        [-4, 54, 16, -2],
        [-6, 46, 28, -4],
        [-4, 36, 36, -4],
        [-4, 28, 46, -6],
        [-2, 16, 54, -4],
        [-2, 10, 58, -2],
    ];
    if frac_x == 0 && frac_y == 0 {
        return clip_sample(i64::from(plane.get_clipped(x, y)), bit_depth);
    }
    let horizontal = |xx: i32, yy: i32| -> i64 {
        FILTERS[frac_x as usize]
            .iter()
            .enumerate()
            .map(|(i, &c)| i64::from(c) * i64::from(plane.get_clipped(xx + i as i32 - 1, yy)))
            .sum::<i64>()
            >> 6
    };
    let value = if frac_y == 0 {
        horizontal(x, y)
    } else if frac_x == 0 {
        FILTERS[frac_y as usize]
            .iter()
            .enumerate()
            .map(|(i, &c)| i64::from(c) * i64::from(plane.get_clipped(x, y + i as i32 - 1)))
            .sum::<i64>()
            >> 6
    } else {
        let values: Vec<i64> = (-1..=2).map(|offset| horizontal(x, y + offset)).collect();
        FILTERS[frac_y as usize]
            .iter()
            .enumerate()
            .map(|(i, &c)| i64::from(c) * values[i])
            .sum::<i64>()
            >> 6
    };
    clip_sample(value, bit_depth)
}
