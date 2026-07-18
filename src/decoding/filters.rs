use super::clip_sample;
use super::SamplePlane;

/// Orientation of an edge processed by the deblocking filter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeDirection {
    /// Vertical edge, processed left-to-right.
    Vertical,
    /// Horizontal edge, processed top-to-bottom.
    Horizontal,
}

/// Inputs to the luma/chroma edge filtering processes in §8.7.2.5.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeblockingParameters {
    /// Boundary strength (0, 1 or 2).
    pub boundary_strength: u8,
    /// Bit depth of the plane.
    pub bit_depth: u8,
    /// Slice beta offset in half-QP units.
    pub beta_offset_div2: i32,
    /// Slice tc offset in half-QP units.
    pub tc_offset_div2: i32,
    /// Whether strong filtering is permitted for the edge.
    pub strong_filtering: bool,
}

/// Applies one 4-sample edge section of the H.265 deblocking filter.
///
/// The caller invokes this in the normative vertical-then-horizontal order
/// and supplies the boundary strength derived from coding-unit metadata.
pub fn apply_deblocking_edge(
    plane: &mut SamplePlane,
    x: i32,
    y: i32,
    direction: EdgeDirection,
    parameters: DeblockingParameters,
) {
    if parameters.boundary_strength == 0 {
        return;
    }
    let scale = 1_i32 << parameters.bit_depth.saturating_sub(8);
    let beta = (13 + 2 * parameters.beta_offset_div2).max(0) * scale;
    let tc = (parameters.boundary_strength as i32 + 1 + parameters.tc_offset_div2).max(1) * scale;
    for section in 0..4_i32 {
        let (px, py) = match direction {
            EdgeDirection::Vertical => (x, y + section),
            EdgeDirection::Horizontal => (x + section, y),
        };
        let (dx, dy) = match direction {
            EdgeDirection::Vertical => (1, 0),
            EdgeDirection::Horizontal => (0, 1),
        };
        let p0 = plane.get_clipped(px - dx, py - dy);
        let q0 = plane.get_clipped(px, py);
        let p1 = plane.get_clipped(px - 2 * dx, py - 2 * dy);
        let q1 = plane.get_clipped(px + dx, py + dy);
        let p2 = plane.get_clipped(px - 3 * dx, py - 3 * dy);
        let q2 = plane.get_clipped(px + 2 * dx, py + 2 * dy);
        let dpq = (p2 - 2 * p1 + p0).abs() + (q2 - 2 * q1 + q0).abs();
        if parameters.strong_filtering
            && parameters.boundary_strength == 2
            && dpq < beta / 4
            && (p0 - q0).abs() < (5 * tc + 1) / 2
        {
            let p0n = clip_sample(
                i64::from(p0 - 2 * tc)
                    .max(i64::from(p2 + 2 * p1 + 2 * p0 + 2 * q0 + q1 + 4) / 8)
                    .min(i64::from(p0 + 2 * tc)),
                parameters.bit_depth,
            );
            let q0n = clip_sample(
                i64::from(q0 - 2 * tc)
                    .max(i64::from(p1 + 2 * p0 + 2 * q0 + 2 * q1 + q2 + 4) / 8)
                    .min(i64::from(q0 + 2 * tc)),
                parameters.bit_depth,
            );
            plane.set(px - dx, py - dy, p0n);
            plane.set(px, py, q0n);
        } else if dpq < beta {
            let delta = (((9 * (q0 - p0) - 3 * (q1 - p1) + 8) >> 4).clamp(-tc, tc)) as i64;
            plane.set(
                px - dx,
                py - dy,
                clip_sample(i64::from(p0) + delta, parameters.bit_depth),
            );
            plane.set(
                px,
                py,
                clip_sample(i64::from(q0) - delta, parameters.bit_depth),
            );
        }
    }
}

/// SAO mode from §8.7.3.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaoType {
    /// No offset.
    None,
    /// Band offset.
    Band,
    /// Edge offset.
    Edge,
}

/// SAO parameters for one CTB and component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaoBlock {
    /// SAO type.
    pub kind: SaoType,
    /// Band position for band-offset mode.
    pub band_position: u8,
    /// Edge class 0..3 for edge-offset mode.
    pub edge_class: u8,
    /// Offset table. Band mode uses entries 1..4; edge mode uses 0..4.
    pub offsets: [i32; 5],
    /// Component bit depth.
    pub bit_depth: u8,
}

/// Applies SAO to a rectangular CTB, returning a separate output plane as in
/// the copy-before-modification rule of §8.7.3.1.
pub fn apply_sao_ctb(
    input: &SamplePlane,
    block_x: u32,
    block_y: u32,
    width: u32,
    height: u32,
    parameters: &SaoBlock,
) -> SamplePlane {
    let mut output = input.clone();
    for y in 0..height {
        for x in 0..width {
            let px = block_x as i32 + x as i32;
            let py = block_y as i32 + y as i32;
            let sample = input.get_clipped(px, py);
            let offset_index = match parameters.kind {
                SaoType::None => 0,
                SaoType::Band => {
                    let band =
                        (sample.max(0) as u32 >> parameters.bit_depth.saturating_sub(5)) & 31;
                    let distance = (band + 32 - u32::from(parameters.band_position)) & 31;
                    if (1..=4).contains(&distance) {
                        distance as usize
                    } else {
                        0
                    }
                }
                SaoType::Edge => {
                    let (h0, v0, h1, v1) = match parameters.edge_class & 3 {
                        0 => (-1, 0, 1, 0),
                        1 => (0, -1, 0, 1),
                        2 => (-1, -1, 1, 1),
                        _ => (1, -1, -1, 1),
                    };
                    if input.get(px + h0, py + v0).is_none()
                        || input.get(px + h1, py + v1).is_none()
                    {
                        0
                    } else {
                        let first = (sample - input.get_clipped(px + h0, py + v0)).signum();
                        let second = (sample - input.get_clipped(px + h1, py + v1)).signum();
                        let edge = 2 + first + second;
                        match edge {
                            0 => 1,
                            1 => 2,
                            2 => 0,
                            _ => edge.clamp(0, 4) as usize,
                        }
                    }
                }
            };
            output.set(
                px,
                py,
                clip_sample(
                    i64::from(sample) + i64::from(parameters.offsets[offset_index]),
                    parameters.bit_depth,
                ),
            );
        }
    }
    output
}
