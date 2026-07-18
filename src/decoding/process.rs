use super::{
    apply_deblocking_edge, apply_sao_ctb, derive_picture_order_count, inter_predict, intra_predict,
    inverse_transform, reconstruct_block, scale_transform_coefficients, DeblockingParameters,
    DecodedPicture, DecodedPictureBuffer, EdgeDirection, IntraPredictionMode, IntraReferences,
    PictureMarking, PredictionLists, ReferenceSet, SamplePlane, SaoBlock, TransformParameters,
};
use crate::PaletteRunSyntax;
use crate::{Block, PictureFormat};

/// Mutable state carried between coded pictures by §§8.1–8.3.
#[derive(Clone, Debug)]
pub struct DecoderState {
    /// Decoded-picture buffer.
    pub dpb: DecodedPictureBuffer,
    /// Previous temporal-layer-zero POC LSB.
    pub previous_poc_lsb: u64,
    /// Previous temporal-layer-zero POC MSB.
    pub previous_poc_msb: i64,
    /// Whether a previous temporal-layer-zero picture exists.
    pub has_previous_poc: bool,
}

impl DecoderState {
    /// Creates decoder state with a maximum DPB size.
    pub fn new(max_dec_pic_buffering: usize) -> Self {
        Self {
            dpb: DecodedPictureBuffer::new(max_dec_pic_buffering),
            previous_poc_lsb: 0,
            previous_poc_msb: 0,
            has_previous_poc: false,
        }
    }

    /// Derives and remembers the current picture's POC.
    pub fn derive_poc(&mut self, poc_lsb: u64, max_poc_lsb: u64, no_rasl_output: bool) -> i64 {
        let (previous_lsb, previous_msb) = if self.has_previous_poc {
            (self.previous_poc_lsb, self.previous_poc_msb)
        } else {
            (0, 0)
        };
        let poc = derive_picture_order_count(
            poc_lsb,
            previous_lsb,
            previous_msb,
            max_poc_lsb,
            no_rasl_output,
        );
        if !no_rasl_output {
            self.previous_poc_lsb = poc_lsb;
            self.previous_poc_msb = poc - poc_lsb as i64;
            self.has_previous_poc = true;
        }
        poc
    }

    /// Stores a filtered decoded picture as a short-term reference.
    pub fn store_picture(
        &mut self,
        picture: DecodedPicture,
        poc: i64,
        layer_id: u8,
        output_needed: bool,
    ) {
        self.dpb.insert(ReferenceSet {
            picture,
            picture_order_count: poc,
            layer_id,
            marking: PictureMarking::ShortTerm,
            output_needed,
        });
    }
}

/// Inputs needed to reconstruct one coding block and run the in-loop filters.
#[derive(Clone, Debug)]
pub struct PictureDecodeContext {
    /// Picture format.
    pub format: PictureFormat,
    /// CTB size in luma samples.
    pub ctb_size: u32,
    /// Whether SAO is enabled for this operation.
    pub sao_enabled: bool,
}

/// Reconstructs one palette-mode coding block from its run syntax (§8.4.4.2.7).
/// The palette entries and escape values are supplied per component and the
/// returned planes are in palette scan order converted to row-major order.
pub fn decode_palette_block(
    palette_entries: &[Vec<i32>],
    runs: &[PaletteRunSyntax],
    escape_values: &[Vec<u64>],
    width: usize,
    height: usize,
    transpose: bool,
    bit_depth: u8,
) -> Vec<Vec<i32>> {
    let component_count = palette_entries.len().max(1);
    let mut output = vec![vec![0; width * height]; component_count];
    let mut indices = vec![-1_i32; width * height];
    let mut escape_index = 0;
    for run in runs {
        for offset in 0..run.run_length {
            let scan_position = run.scan_position + offset;
            if scan_position >= width * height {
                continue;
            }
            let index = if run.copy_above_indices_flag && scan_position >= width {
                indices[scan_position - width]
            } else {
                run.palette_index.unwrap_or(0) as i32
            };
            indices[scan_position] = index;
            let is_escape =
                index < 0 || index as usize >= palette_entries.first().map_or(0, Vec::len);
            let row = scan_position / width;
            let column = scan_position % width;
            let (x, y) = if transpose {
                (row.min(width - 1), column.min(height - 1))
            } else {
                (column, row)
            };
            let destination = y * width + x;
            for (component, component_output) in output.iter_mut().enumerate().take(component_count)
            {
                component_output[destination] = if is_escape {
                    let value = escape_values
                        .get(component)
                        .and_then(|values| values.get(escape_index))
                        .copied()
                        .unwrap_or(0);
                    super::clip_sample(value as i64, bit_depth)
                } else {
                    palette_entries
                        .get(component)
                        .and_then(|entries| entries.get(index as usize))
                        .copied()
                        .unwrap_or(0)
                };
            }
            if is_escape {
                escape_index += 1;
            }
        }
    }
    output
}

impl PictureDecodeContext {
    /// Creates a context for a picture.
    pub const fn new(format: PictureFormat, ctb_size: u32) -> Self {
        Self {
            format,
            ctb_size,
            sao_enabled: true,
        }
    }

    /// Reconstructs an intra block before loop filtering.
    pub fn reconstruct_intra(
        &self,
        picture: &mut DecodedPicture,
        block: Block,
        mode: IntraPredictionMode,
        levels: &[i32],
        transform: TransformParameters,
    ) {
        let component = transform.component;
        let Some(plane) = picture.plane(component) else {
            return;
        };
        let references = IntraReferences::from_plane(plane, block);
        let prediction = intra_predict(
            &references,
            block.width as usize,
            block.height as usize,
            mode,
            transform.bit_depth,
        );
        let scaled = scale_transform_coefficients(
            levels,
            block.width as usize,
            transform.qp,
            transform.bit_depth,
            None,
        );
        let residual = if transform.transform_bypass {
            scaled
        } else {
            inverse_transform(
                &scaled,
                block.width as usize,
                transform.intra_4x4,
                transform.bit_depth,
            )
        };
        let reconstructed = reconstruct_block(&prediction, &residual, block, transform.bit_depth);
        if let Some(plane) = picture.plane_mut(component) {
            plane.write_block(block, &reconstructed);
        }
    }

    /// Reconstructs an inter block from DPB references before loop filtering.
    pub fn reconstruct_inter(
        &self,
        picture: &mut DecodedPicture,
        block: Block,
        references: &[&DecodedPicture],
        lists: PredictionLists,
        levels: &[i32],
        transform: TransformParameters,
    ) {
        let component_scale = if transform.component == 0 {
            1_usize
        } else {
            self.format.chroma_format.subsampling().0 as usize
        };
        let prediction = inter_predict(
            references,
            block,
            lists,
            self.format.chroma_format,
            transform.component,
            transform.bit_depth,
        );
        let scaled = scale_transform_coefficients(
            levels,
            block.width as usize / component_scale,
            transform.qp,
            transform.bit_depth,
            None,
        );
        let size = block.width as usize / component_scale;
        let residual = if transform.transform_bypass {
            scaled
        } else {
            inverse_transform(&scaled, size, false, transform.bit_depth)
        };
        let reconstructed = reconstruct_block(
            &prediction,
            &residual,
            Block {
                x: block.x
                    / if transform.component == 0 {
                        1
                    } else {
                        self.format.chroma_format.subsampling().0
                    },
                y: block.y
                    / if transform.component == 0 {
                        1
                    } else {
                        self.format.chroma_format.subsampling().1
                    },
                width: size as u32,
                height: size as u32,
            },
            transform.bit_depth,
        );
        if let Some(plane) = picture.plane_mut(transform.component) {
            plane.write_block(
                Block {
                    x: block.x
                        / if transform.component == 0 {
                            1
                        } else {
                            self.format.chroma_format.subsampling().0
                        },
                    y: block.y
                        / if transform.component == 0 {
                            1
                        } else {
                            self.format.chroma_format.subsampling().1
                        },
                    width: size as u32,
                    height: size as u32,
                },
                &reconstructed,
            );
        }
    }

    /// Applies the vertical-then-horizontal deblocking order to a plane.
    pub fn deblock_plane(&self, plane: &mut SamplePlane, parameters: DeblockingParameters) {
        let step = 8_u32;
        for y in (0..plane.height()).step_by(step as usize) {
            for x in (step..plane.width()).step_by(step as usize) {
                apply_deblocking_edge(
                    plane,
                    x as i32,
                    y as i32,
                    EdgeDirection::Vertical,
                    parameters,
                );
            }
        }
        for y in (step..plane.height()).step_by(step as usize) {
            for x in (0..plane.width()).step_by(step as usize) {
                apply_deblocking_edge(
                    plane,
                    x as i32,
                    y as i32,
                    EdgeDirection::Horizontal,
                    parameters,
                );
            }
        }
    }

    /// Applies SAO independently to each CTB, preserving the copy semantics.
    pub fn sao_plane(&self, plane: &SamplePlane, parameters: &SaoBlock) -> SamplePlane {
        let mut output = plane.clone();
        let width = self.ctb_size.min(plane.width());
        let height = self.ctb_size.min(plane.height());
        for y in (0..plane.height()).step_by(self.ctb_size as usize) {
            for x in (0..plane.width()).step_by(self.ctb_size as usize) {
                let filtered = apply_sao_ctb(
                    plane,
                    x,
                    y,
                    width.min(plane.width() - x),
                    height.min(plane.height() - y),
                    parameters,
                );
                for row in 0..filtered.height() {
                    for column in 0..filtered.width() {
                        output.set(
                            x as i32 + column as i32,
                            y as i32 + row as i32,
                            filtered
                                .get(x as i32 + column as i32, y as i32 + row as i32)
                                .unwrap_or(0),
                        );
                    }
                }
            }
        }
        output
    }
}
