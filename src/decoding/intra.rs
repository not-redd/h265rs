use super::{clip_sample, SamplePlane};
use crate::Block;

/// Intra prediction modes from Table 8-1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntraPredictionMode {
    /// `INTRA_PLANAR` (mode 0).
    Planar,
    /// `INTRA_DC` (mode 1).
    Dc,
    /// Angular mode 2..34.
    Angular(u8),
}

impl IntraPredictionMode {
    /// Converts the syntax mode number to the typed representation.
    pub const fn from_number(mode: u8) -> Option<Self> {
        match mode {
            0 => Some(Self::Planar),
            1 => Some(Self::Dc),
            2..=34 => Some(Self::Angular(mode)),
            _ => None,
        }
    }

    /// Returns the syntax mode number.
    pub const fn number(self) -> u8 {
        match self {
            Self::Planar => 0,
            Self::Dc => 1,
            Self::Angular(mode) => mode,
        }
    }
}

/// Reference samples surrounding one intra transform block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntraReferences {
    /// The top-left neighbouring sample.
    pub top_left: i32,
    /// Top reference samples, left-to-right, with length at least `2 * size + 1`.
    pub top: Vec<i32>,
    /// Left reference samples, top-to-bottom, with length at least `2 * size + 1`.
    pub left: Vec<i32>,
}

impl IntraReferences {
    /// Derives substituted reference samples using the nearest available
    /// sample, as specified by §8.4.4.2.2.
    pub fn from_plane(plane: &SamplePlane, block: Block) -> Self {
        let size = block.width.max(block.height) as usize;
        let top_left = plane.get_clipped(block.x as i32 - 1, block.y as i32 - 1);
        let mut top = Vec::with_capacity(size * 2 + 1);
        let mut left = Vec::with_capacity(size * 2 + 1);
        for index in 0..=size * 2 {
            top.push(plane.get_clipped(block.x as i32 + index as i32, block.y as i32 - 1));
            left.push(plane.get_clipped(block.x as i32 - 1, block.y as i32 + index as i32));
        }
        Self {
            top_left,
            top,
            left,
        }
    }

    /// Creates references from explicit top and left arrays.
    pub fn new(top_left: i32, top: Vec<i32>, left: Vec<i32>) -> Self {
        Self {
            top_left,
            top,
            left,
        }
    }
}

/// Derives `IntraPredModeY` from the two neighbouring modes and MPM syntax.
pub fn derive_luma_intra_mode(
    neighbour_a: u8,
    neighbour_b: u8,
    prev_flag: bool,
    mpm_idx: u8,
    rem: u8,
) -> u8 {
    let a = if neighbour_a <= 34 { neighbour_a } else { 1 };
    let b = if neighbour_b <= 34 { neighbour_b } else { 1 };
    let mut candidates = if a == b {
        if a < 2 {
            [0, 1, 26]
        } else {
            [a, 2 + (a + 29) % 32, 2 + (a - 2 + 1) % 32]
        }
    } else {
        let third = if a != 0 && b != 0 {
            0
        } else if a != 1 && b != 1 {
            1
        } else {
            26
        };
        [a, b, third]
    };
    if prev_flag {
        return candidates[(mpm_idx.min(2)) as usize];
    }
    candidates.sort_unstable();
    let mut mode = rem.min(31);
    for candidate in candidates {
        if mode >= candidate {
            mode = mode.saturating_add(1);
        }
    }
    mode.min(34)
}

/// Derives `IntraPredModeC` from the chroma syntax mode and luma mode.
pub fn derive_chroma_intra_mode(intra_chroma_pred_mode: u8, luma_mode: u8, yuv422: bool) -> u8 {
    let mode_idx = match intra_chroma_pred_mode {
        0 => match luma_mode {
            0 => 34,
            26 => 0,
            10 => 0,
            1 => 0,
            _ => 0,
        },
        1 => match luma_mode {
            0 => 26,
            26 => 34,
            10 => 26,
            1 => 26,
            _ => 26,
        },
        2 => match luma_mode {
            0 => 10,
            26 => 10,
            10 => 34,
            1 => 10,
            _ => 10,
        },
        3 => match luma_mode {
            0 => 1,
            26 => 1,
            10 => 1,
            1 => 34,
            _ => 1,
        },
        _ => luma_mode,
    };
    if !yuv422 {
        return mode_idx;
    }
    const MAP: [u8; 35] = [
        0, 1, 2, 2, 2, 2, 3, 5, 7, 8, 10, 12, 13, 15, 17, 18, 19, 20, 21, 22, 23, 23, 24, 24, 25,
        25, 26, 27, 27, 28, 28, 29, 29, 30, 31,
    ];
    if mode_idx <= 34 {
        MAP[mode_idx as usize]
    } else {
        1
    }
}

/// Generates an intra prediction block according to §§8.4.4.2.4–8.4.4.2.6.
pub fn intra_predict(
    references: &IntraReferences,
    width: usize,
    height: usize,
    mode: IntraPredictionMode,
    bit_depth: u8,
) -> Vec<i32> {
    let mut output = vec![0; width * height];
    match mode {
        IntraPredictionMode::Planar => {
            for y in 0..height {
                for x in 0..width {
                    let hor = (width - 1 - x) as i64 * i64::from(references.left[y])
                        + (x + 1) as i64 * i64::from(references.top[width]);
                    let ver = (height - 1 - y) as i64 * i64::from(references.top[x])
                        + (y + 1) as i64 * i64::from(references.left[height]);
                    output[y * width + x] = ((hor + ver + width.max(height) as i64)
                        / (width.max(height) as i64 * 2))
                        as i32;
                }
            }
        }
        IntraPredictionMode::Dc => {
            let count = width + height;
            let sum: i64 = references
                .top
                .iter()
                .take(width)
                .map(|&v| i64::from(v))
                .sum::<i64>()
                + references
                    .left
                    .iter()
                    .take(height)
                    .map(|&v| i64::from(v))
                    .sum::<i64>();
            let dc = ((sum + (count / 2) as i64) / count as i64) as i32;
            output.fill(dc);
        }
        IntraPredictionMode::Angular(mode) => {
            angular_predict(references, width, height, mode, bit_depth, &mut output)
        }
    }
    output
}

fn angular_predict(
    refs: &IntraReferences,
    width: usize,
    height: usize,
    mode: u8,
    bit_depth: u8,
    output: &mut [i32],
) {
    const ANGLES: [i32; 35] = [
        0, 0, 32, 26, 21, 17, 13, 9, 5, 2, 0, -2, -5, -9, -13, -17, -21, -26, -32, -26, -21, -17,
        -13, -9, -5, -2, 0, 2, 5, 9, 13, 17, 21, 26, 32,
    ];
    let angle = ANGLES[mode.clamp(2, 34) as usize];
    let vertical = mode >= 18;
    for y in 0..height {
        for x in 0..width {
            let distance = if vertical { y as i32 + 1 } else { x as i32 + 1 };
            let delta = distance * angle;
            let index = (delta >> 5) - 1;
            let frac = delta & 31;
            let source = if vertical { &refs.top } else { &refs.left };
            let perpendicular = if vertical { y } else { x };
            let sample = if frac == 0 {
                ref_at(source, index + perpendicular as i32 + 1, refs.top_left)
            } else {
                let first = ref_at(source, index + perpendicular as i32 + 1, refs.top_left);
                let second = ref_at(source, index + perpendicular as i32 + 2, refs.top_left);
                ((32 - frac) * first + frac * second + 16) >> 5
            };
            output[y * width + x] = clip_sample(i64::from(sample), bit_depth);
        }
    }
    // §8.4.4.2.6's boundary filter for the exact horizontal/vertical modes.
    if width == height && width < 32 && (mode == 10 || mode == 26) {
        if mode == 26 {
            for y in 0..height {
                output[y * width] = clip_sample(
                    i64::from(refs.top[0]) + i64::from(refs.left[y] - refs.top_left) / 2,
                    bit_depth,
                );
            }
        } else {
            for (x, sample) in output.iter_mut().enumerate().take(width) {
                *sample = clip_sample(
                    i64::from(refs.left[0]) + i64::from(refs.top[x] - refs.top_left) / 2,
                    bit_depth,
                );
            }
        }
    }
}

fn ref_at(reference: &[i32], index: i32, fallback: i32) -> i32 {
    if index < 0 {
        fallback
    } else {
        reference
            .get(index as usize)
            .copied()
            .unwrap_or_else(|| *reference.last().unwrap_or(&fallback))
    }
}
