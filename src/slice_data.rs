use crate::syntax::{
    BitReader, CabacContextTable, SliceSegmentHeaderContext, SliceSegmentHeaderSyntax, SyntaxError,
};

/// Interface required by Clause 7 for `ae(v)` syntax elements.
///
/// Clause 9 supplies the CABAC engine. Keeping that engine behind this trait
/// lets the Clause 7 syntax layer describe and validate the coded structure
/// without coupling it to a particular arithmetic-decoder implementation.
pub trait CabacReader {
    /// Reads one context-adaptive arithmetic-coded syntax value.
    fn read_ae(&mut self) -> Result<u64, SyntaxError>;

    /// Reads one context-adaptive bin at an explicit Clause 9 context index.
    /// Structural readers may use their single-value fallback.
    fn read_ae_context(&mut self, _ctx_idx: usize) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads one bin from a named Clause 9 context table and local index.
    /// Structural readers retain the single-bin fallback; a real CABAC
    /// reader maps the pair into its flat context storage.
    fn read_ae_named(
        &mut self,
        _table: CabacContextTable,
        _ctx_idx: usize,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads a fixed-length PCM sample or other bypass-coded value.
    ///
    /// Clause 9 implementations can override this method. The default keeps
    /// the structural Clause 7 parser usable without pretending that a
    /// CABAC-only reader can decode bypass bits.
    fn read_bits(&mut self, _count: usize) -> Result<u64, SyntaxError> {
        Err(SyntaxError::ArithmeticCodingUnsupported)
    }

    /// Reads PCM samples from the CABAC-owned bitstream and reinitializes the
    /// arithmetic engine after the samples, as required by §9.3.2.6.
    fn read_pcm_samples(
        &mut self,
        _luma_sample_count: usize,
        _chroma_sample_count: usize,
        _bit_depth_luma: usize,
        _bit_depth_chroma: usize,
    ) -> Result<PcmSampleSyntax, SyntaxError> {
        Err(SyntaxError::ArithmeticCodingUnsupported)
    }

    /// Reads one bypass-coded CABAC bin.
    fn read_bypass_bin(&mut self) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads multiple bypass-coded bins, most-significant bit first.
    fn read_bypass_bits(&mut self, count: usize) -> Result<u64, SyntaxError> {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | self.read_bypass_bin()?;
        }
        Ok(value)
    }

    /// Reads a truncated-Rice coded value using bypass bins.
    fn read_truncated_rice(
        &mut self,
        _c_max: u64,
        _rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads a truncated-Rice value whose initial unary bins are
    /// context-coded and remaining bins are bypass-coded.
    fn read_truncated_rice_context(
        &mut self,
        _table: CabacContextTable,
        _ctx_idx: usize,
        _context_bins: usize,
        c_max: u64,
        rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        self.read_truncated_rice(c_max, rice_parameter)
    }

    /// Reads a truncated-Rice value whose initial bins all use one context
    /// increment, followed by bypass bins.
    fn read_truncated_rice_repeated_context(
        &mut self,
        _table: CabacContextTable,
        _ctx_inc: usize,
        _context_bins: usize,
        c_max: u64,
        rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        self.read_truncated_rice(c_max, rice_parameter)
    }

    /// Reads a fixed-length coded value using bypass bins.
    fn read_fixed_bypass(&mut self, _c_max: u64) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads the context/bypass-coded `part_mode` bin string.
    fn read_part_mode(
        &mut self,
        _is_intra: bool,
        _at_minimum_size: bool,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `part_mode` with the SPS `amp_enabled_flag` available.
    fn read_part_mode_with_amp(
        &mut self,
        is_intra: bool,
        at_minimum_size: bool,
        amp_enabled: bool,
    ) -> Result<u64, SyntaxError> {
        let _ = amp_enabled;
        self.read_part_mode(is_intra, at_minimum_size)
    }

    /// Reads the context/bypass-coded `inter_pred_idc` bin string.
    fn read_inter_pred_idc(
        &mut self,
        _n_pb_w: u64,
        _n_pb_h: u64,
        _ct_depth: usize,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `cu_qp_delta_abs`, including its EG0 suffix when present.
    fn read_cu_qp_delta_abs(&mut self) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `coeff_abs_level_remaining` with the default Clause 9 Rice state.
    fn read_coeff_abs_level_remaining(&mut self, _base_level: u64) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `coeff_abs_level_remaining()` with the Rice parameter derived
    /// from the preceding invocation in the current sub-block.
    fn read_coeff_abs_level_remaining_with_parameters(
        &mut self,
        base_level: u64,
        _rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        self.read_coeff_abs_level_remaining(base_level)
    }

    /// Reads coefficient-remaining syntax with the extended-precision and
    /// Rice-state options from §9.3.3.11.
    fn read_coeff_abs_level_remaining_with_options(
        &mut self,
        base_level: u64,
        rice_parameter: u8,
        _extended_precision: bool,
        _log2_transform_range: u8,
    ) -> Result<u64, SyntaxError> {
        self.read_coeff_abs_level_remaining_with_parameters(base_level, rice_parameter)
    }

    /// Applies CABAC bypass alignment before sign and remaining coefficient
    /// bins when the active extension requests it.
    fn cabac_bypass_alignment(&mut self) -> Result<(), SyntaxError> {
        Ok(())
    }

    /// Reads an order-`k` Exp-Golomb value using bypass bins.
    fn read_exp_golomb(&mut self, _order: u8) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads a limited EGk value for extended-precision residual coding.
    fn read_limited_exp_golomb(
        &mut self,
        _rice_parameter: u8,
        _log2_transform_range: u8,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads a palette value using §9.3.3.12.
    fn read_palette_value(
        &mut self,
        _bit_depth: usize,
        _cu_transquant_bypass_flag: bool,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `num_palette_indices_minus1` using §9.3.3.14.
    fn read_palette_num_indices(&mut self, _max_palette_index: u64) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `palette_idx_idc` using truncated-binary bypass bins.
    fn read_palette_index(
        &mut self,
        _max_palette_index: u64,
        _first: bool,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads `palette_run_prefix` and its context-derived unary bins.
    fn read_palette_run_prefix(
        &mut self,
        _palette_max_run_minus1: u64,
        _palette_index: u64,
        _copy_above: bool,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads the optional truncated-binary `palette_run_suffix`.
    fn read_palette_run_suffix(
        &mut self,
        _palette_max_run_minus1: u64,
        _palette_run_prefix: u64,
    ) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads a CABAC termination bin (`ctxTable == 0`, `ctxIdx == 0`).
    /// The default preserves compatibility with structural test readers.
    fn read_terminate(&mut self) -> Result<u64, SyntaxError> {
        self.read_ae()
    }

    /// Reads an `rbsp`-level byte-alignment bit sequence after CABAC syntax.
    fn byte_alignment(&mut self) -> Result<(), SyntaxError>;

    /// Initializes the arithmetic engine for the next CABAC substream after
    /// a tile/WPP byte-alignment boundary.
    fn initialize_arithmetic_engine(&mut self) -> Result<(), SyntaxError> {
        Ok(())
    }

    /// Restores the context variables for the first CTU of a new tile.
    fn reset_contexts_to_initial(&mut self) {}

    /// Stores adapted contexts for the next entropy-synchronized CTU row.
    fn store_wpp_contexts(&mut self) {}

    /// Restores adapted contexts from the preceding entropy-synchronized row.
    /// Returns `true` when a stored state was available.
    fn synchronize_wpp_contexts(&mut self) -> bool {
        false
    }

    /// Finishes `rbsp_slice_segment_trailing_bits()` and returns the number of
    /// consumed `cabac_zero_word` values.
    fn rbsp_slice_segment_trailing_bits(&mut self) -> Result<usize, SyntaxError> {
        Err(SyntaxError::ArithmeticCodingUnsupported)
    }
}

/// Structural coding-quadtree node from §7.3.8.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodingQuadtreeNode {
    /// Luma-space x coordinate of the coding block.
    pub x: u64,
    /// Luma-space y coordinate of the coding block.
    pub y: u64,
    /// `log2CbSize` for this node.
    pub log2_cb_size: u8,
    /// `cqtDepth` for this node.
    pub cqt_depth: u32,
    /// `split_cu_flag`, inferred false when the syntax is not present.
    pub split_cu_flag: bool,
    /// Child quadrants in standard coding order.
    pub children: Vec<CodingQuadtreeNode>,
}

/// Picture geometry required by `coding_quadtree()` split inference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodingQuadtreeGeometry {
    /// `pic_width_in_luma_samples`.
    pub pic_width_in_luma_samples: u64,
    /// `pic_height_in_luma_samples`.
    pub pic_height_in_luma_samples: u64,
    /// `MinCbLog2SizeY`.
    pub min_cb_log2_size: u8,
}

/// Context for `prediction_unit()` syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredictionUnitContext {
    /// Slice type: 0 = B, 1 = P, 2 = I.
    pub slice_type: u64,
    /// `num_ref_idx_l0_active_minus1`.
    pub num_ref_idx_l0_active_minus1: u64,
    /// `num_ref_idx_l1_active_minus1`.
    pub num_ref_idx_l1_active_minus1: u64,
    /// `five_minus_max_num_merge_cand`.
    pub five_minus_max_num_merge_cand: u64,
    /// `mvd_l1_zero_flag`.
    pub mvd_l1_zero_flag: bool,
}

/// Parsed `prediction_unit()` syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredictionUnitSyntax {
    /// `merge_flag` for non-skipped coding units.
    pub merge_flag: Option<bool>,
    /// `merge_idx`, when present.
    pub merge_idx: Option<u64>,
    /// `inter_pred_idc`, for B slices and non-merge inter units.
    pub inter_pred_idc: Option<u64>,
    /// `ref_idx_l0`, when present.
    pub ref_idx_l0: Option<u64>,
    /// L0 motion-vector difference syntax.
    pub mvd_l0: Option<MotionVectorDifferenceSyntax>,
    /// `mvp_l0_flag`, when L0 prediction is used.
    pub mvp_l0_flag: Option<bool>,
    /// `ref_idx_l1`, when present.
    pub ref_idx_l1: Option<u64>,
    /// L1 motion-vector difference syntax, unless inferred zero.
    pub mvd_l1: Option<MotionVectorDifferenceSyntax>,
    /// `mvp_l1_flag`, when L1 prediction is used.
    pub mvp_l1_flag: Option<bool>,
}

/// Context for the conditional syntax in `coding_unit()`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodingUnitContext {
    /// Slice type: 0 = B, 1 = P, 2 = I.
    pub slice_type: u64,
    /// `transquant_bypass_enabled_flag`.
    pub transquant_bypass_enabled_flag: bool,
    /// `cu_qp_delta_enabled_flag`.
    pub cu_qp_delta_enabled_flag: bool,
    /// `cu_chroma_qp_offset_enabled_flag`.
    pub cu_chroma_qp_offset_enabled_flag: bool,
    /// `palette_mode_enabled_flag`.
    pub palette_mode_enabled_flag: bool,
    /// `pcm_enabled_flag`.
    pub pcm_enabled_flag: bool,
    /// `log2CbSize`.
    pub log2_cb_size: u8,
    /// `MinCbLog2SizeY`.
    pub min_cb_log2_size: u8,
    /// `Log2MinIpcmCbSizeY`.
    pub log2_min_ipcm_cb_size: u8,
    /// `Log2MaxIpcmCbSizeY`.
    pub log2_max_ipcm_cb_size: u8,
    /// `MaxTbLog2SizeY`.
    pub max_tb_log2_size: u8,
    /// `ChromaArrayType`.
    pub chroma_array_type: u8,
    /// `palette_max_size` from the active palette tools.
    pub palette_max_size: u64,
    /// Number of predictor palette entries available to this CU.
    pub predictor_palette_size: usize,
    /// `chroma_qp_offset_list_len_minus1`.
    pub chroma_qp_offset_list_len_minus1: u64,
    /// Prediction-unit context for inter coding units.
    pub prediction: PredictionUnitContext,
}

/// Intra prediction-mode syntax from a coding unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntraPredictionSyntax {
    /// `prev_intra_luma_pred_flag` values.
    pub prev_luma_pred_flags: Vec<bool>,
    /// `mpm_idx` values, when the previous flag is true.
    pub mpm_idx: Vec<Option<u64>>,
    /// `rem_intra_luma_pred_mode` values, when the previous flag is false.
    pub rem_intra_luma_pred_mode: Vec<Option<u64>>,
    /// `intra_chroma_pred_mode` values.
    pub chroma_pred_modes: Vec<u64>,
}

/// Parsed coding-unit syntax through `rqt_root_cbf`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodingUnitSyntax {
    /// `cu_transquant_bypass_flag`, when present.
    pub cu_transquant_bypass_flag: Option<bool>,
    /// `cu_skip_flag`, when present.
    pub cu_skip_flag: Option<bool>,
    /// `pred_mode_flag`, when present.
    pub pred_mode_flag: Option<bool>,
    /// `palette_mode_flag`, when present.
    pub palette_mode_flag: Option<bool>,
    /// Palette syntax for a palette-coded CU.
    pub palette_coding: Option<PaletteCodingSyntax>,
    /// `part_mode`, inferred as 2Nx2N when not signalled.
    pub part_mode: u64,
    /// `pcm_flag`, when its condition is met.
    pub pcm_flag: Option<bool>,
    /// Intra prediction syntax, for non-PCM intra coding units.
    pub intra_prediction: Option<IntraPredictionSyntax>,
    /// Prediction units for inter or skipped coding units.
    pub prediction_units: Vec<PredictionUnitSyntax>,
    /// `rqt_root_cbf`, when present.
    pub rqt_root_cbf: Option<bool>,
    /// True when a caller must parse `transform_tree()` next.
    pub transform_tree_required: bool,
}

fn prediction_unit_count(slice_type: u64, part_mode: u64) -> usize {
    if slice_type == 2 {
        usize::from(part_mode == 1).max(1)
    } else {
        match part_mode {
            0 => 1,
            1 | 2 | 4 | 5 | 6 | 7 => 2,
            3 => 4,
            _ => 1,
        }
    }
}

fn prediction_unit_dimensions(
    block_size: u64,
    part_mode: u64,
    unit_index: usize,
) -> Option<(u64, u64)> {
    let half = block_size / 2;
    let quarter = block_size / 4;
    match part_mode {
        0 => Some((block_size, block_size)),
        1 => Some((block_size, half)),
        2 => Some((half, block_size)),
        3 => Some((half, half)),
        4 | 5 => Some((
            block_size,
            if unit_index == 0 {
                quarter
            } else {
                block_size - quarter
            },
        )),
        6 | 7 => Some((
            if unit_index == 0 {
                quarter
            } else {
                block_size - quarter
            },
            block_size,
        )),
        _ => None,
    }
}

/// Parses `coding_unit()` through its root-CBF decision.
///
/// The returned `transform_tree_required` flag identifies the exact Clause 7
/// boundary at which the transform-tree parser must continue. PCM and palette
/// branches are represented in the result and likewise remain explicit for
/// their dedicated syntax parsers.
pub fn parse_coding_unit(
    cabac: &mut impl CabacReader,
    context: CodingUnitContext,
) -> Result<CodingUnitSyntax, SyntaxError> {
    parse_coding_unit_with_skip_context_and_bit_depth(cabac, context, 0, 8, 8, 0, false)
}

/// Parses one coding unit with the SPS `amp_enabled_flag` supplied to the
/// Clause 9 `part_mode` binarization.
pub fn parse_coding_unit_with_amp(
    cabac: &mut impl CabacReader,
    context: CodingUnitContext,
    amp_enabled: bool,
) -> Result<CodingUnitSyntax, SyntaxError> {
    parse_coding_unit_with_skip_context_and_bit_depth(cabac, context, 0, 8, 8, 0, amp_enabled)
}

fn parse_coding_unit_with_skip_context_and_bit_depth(
    cabac: &mut impl CabacReader,
    context: CodingUnitContext,
    skip_context_increment: usize,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
    cqt_depth: u32,
    amp_enabled: bool,
) -> Result<CodingUnitSyntax, SyntaxError> {
    let cu_transquant_bypass_flag = if context.transquant_bypass_enabled_flag {
        Some(cabac.read_ae_named(CabacContextTable::CuTransquantBypass, 0)? != 0)
    } else {
        None
    };
    let cu_skip_flag = if context.slice_type != 2 {
        Some(cabac.read_ae_named(CabacContextTable::CuSkip, skip_context_increment)? != 0)
    } else {
        None
    };
    if cu_skip_flag == Some(true) {
        let prediction_units = vec![parse_prediction_unit(cabac, context.prediction, true)?];
        return Ok(CodingUnitSyntax {
            cu_transquant_bypass_flag,
            cu_skip_flag,
            pred_mode_flag: None,
            palette_mode_flag: None,
            palette_coding: None,
            part_mode: 0,
            pcm_flag: None,
            intra_prediction: None,
            prediction_units,
            rqt_root_cbf: None,
            transform_tree_required: false,
        });
    }
    let pred_mode_flag = if context.slice_type != 2 {
        Some(cabac.read_ae_named(CabacContextTable::PredMode, 0)? != 0)
    } else {
        None
    };
    let is_intra = pred_mode_flag != Some(false);
    let palette_mode_flag = if context.palette_mode_enabled_flag
        && is_intra
        && context.log2_cb_size <= context.max_tb_log2_size
    {
        Some(cabac.read_ae_named(CabacContextTable::PaletteMode, 0)? != 0)
    } else {
        None
    };
    let part_mode = if palette_mode_flag == Some(true) {
        0
    } else if !is_intra || context.log2_cb_size == context.min_cb_log2_size {
        cabac.read_part_mode_with_amp(
            is_intra,
            context.log2_cb_size == context.min_cb_log2_size,
            amp_enabled,
        )?
    } else {
        0
    };
    if palette_mode_flag == Some(true) {
        return Ok(CodingUnitSyntax {
            cu_transquant_bypass_flag,
            cu_skip_flag,
            pred_mode_flag,
            palette_mode_flag,
            palette_coding: Some(parse_palette_coding_with_bit_depth(
                cabac,
                PaletteCodingContext {
                    n_cb_s: 1usize.checked_shl(u32::from(context.log2_cb_size)).ok_or(
                        SyntaxError::InvalidSyntaxValue("palette coding block size is too large"),
                    )?,
                    predictor_palette_size: context.predictor_palette_size,
                    palette_max_size: context.palette_max_size,
                    chroma_array_type: context.chroma_array_type,
                    cu_qp_delta_enabled_flag: context.cu_qp_delta_enabled_flag,
                    cu_chroma_qp_offset_enabled_flag: context.cu_chroma_qp_offset_enabled_flag,
                    chroma_qp_offset_list_len_minus1: context.chroma_qp_offset_list_len_minus1,
                    cu_transquant_bypass_flag: cu_transquant_bypass_flag == Some(true),
                },
                bit_depth_luma,
                bit_depth_chroma,
            )?),
            part_mode,
            pcm_flag: None,
            intra_prediction: None,
            prediction_units: Vec::new(),
            rqt_root_cbf: None,
            transform_tree_required: false,
        });
    }
    let pcm_flag = if is_intra
        && part_mode == 0
        && context.pcm_enabled_flag
        && context.log2_cb_size >= context.log2_min_ipcm_cb_size
        && context.log2_cb_size <= context.log2_max_ipcm_cb_size
    {
        Some(cabac.read_terminate()? != 0)
    } else {
        None
    };
    if pcm_flag == Some(true) {
        return Ok(CodingUnitSyntax {
            cu_transquant_bypass_flag,
            cu_skip_flag,
            pred_mode_flag,
            palette_mode_flag,
            palette_coding: None,
            part_mode,
            pcm_flag,
            intra_prediction: None,
            prediction_units: Vec::new(),
            rqt_root_cbf: None,
            transform_tree_required: false,
        });
    }
    let intra_prediction = if is_intra {
        let block_size = 1usize.checked_shl(u32::from(context.log2_cb_size)).ok_or(
            SyntaxError::InvalidSyntaxValue("intra block size is too large"),
        )?;
        let partition_size = if part_mode == 1 {
            block_size / 2
        } else {
            block_size
        };
        let partition_count = (block_size / partition_size) * (block_size / partition_size);
        let mut prev_luma_pred_flags = Vec::with_capacity(partition_count);
        let mut mpm_idx = Vec::with_capacity(partition_count);
        let mut rem_intra_luma_pred_mode = Vec::with_capacity(partition_count);
        for _ in 0..partition_count {
            let prev = cabac.read_ae_named(CabacContextTable::PrevIntraLumaPred, 0)? != 0;
            prev_luma_pred_flags.push(prev);
            if prev {
                mpm_idx.push(Some(cabac.read_truncated_rice(2, 0)?));
                rem_intra_luma_pred_mode.push(None);
            } else {
                mpm_idx.push(None);
                let mode = cabac.read_fixed_bypass(31)?;
                rem_intra_luma_pred_mode.push(Some(mode));
            }
        }
        let chroma_count = if context.chroma_array_type == 3 {
            partition_count
        } else if context.chroma_array_type != 0 {
            1
        } else {
            0
        };
        let mut chroma_pred_modes = Vec::with_capacity(chroma_count);
        for _ in 0..chroma_count {
            let first = cabac.read_ae_named(CabacContextTable::IntraChromaPred, 0)?;
            if first == 0 {
                chroma_pred_modes.push(4);
            } else {
                let suffix = cabac.read_bypass_bits(2)?;
                chroma_pred_modes.push(suffix);
            }
        }
        Some(IntraPredictionSyntax {
            prev_luma_pred_flags,
            mpm_idx,
            rem_intra_luma_pred_mode,
            chroma_pred_modes,
        })
    } else {
        None
    };
    let prediction_units = if is_intra {
        Vec::new()
    } else {
        let count = prediction_unit_count(context.slice_type, part_mode);
        let cb_size = block_size(context.log2_cb_size)?;
        let mut units = Vec::with_capacity(count);
        for unit_index in 0..count {
            let dimensions = prediction_unit_dimensions(cb_size, part_mode, unit_index);
            units.push(match dimensions {
                Some((n_pb_w, n_pb_h)) => parse_prediction_unit_with_dimensions(
                    cabac,
                    context.prediction,
                    false,
                    n_pb_w,
                    n_pb_h,
                    cqt_depth as usize,
                )?,
                None => parse_prediction_unit(cabac, context.prediction, false)?,
            });
        }
        units
    };
    let has_merge_2nx2n = prediction_units
        .first()
        .is_some_and(|unit| unit.merge_flag == Some(true))
        && part_mode == 0;
    let rqt_root_cbf = if !is_intra && !has_merge_2nx2n {
        Some(cabac.read_ae_named(CabacContextTable::RqtRootCbf, 0)? != 0)
    } else {
        None
    };
    Ok(CodingUnitSyntax {
        cu_transquant_bypass_flag,
        cu_skip_flag,
        pred_mode_flag,
        palette_mode_flag,
        palette_coding: None,
        part_mode,
        pcm_flag,
        intra_prediction,
        prediction_units,
        transform_tree_required: rqt_root_cbf.unwrap_or(true),
        rqt_root_cbf,
    })
}

/// Context controlling the conditional branches of `transform_tree()` and
/// `transform_unit()`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransformTreeContext {
    /// Whether the coding unit uses intra prediction.
    pub cu_pred_mode_intra: bool,
    /// `ChromaArrayType`.
    pub chroma_array_type: u8,
    /// `MinTbLog2SizeY`.
    pub min_tb_log2_size: u8,
    /// `MaxTbLog2SizeY`.
    pub max_tb_log2_size: u8,
    /// `MaxTrafoDepth` for this coding unit.
    pub max_trafo_depth: u32,
    /// `IntraSplitFlag`.
    pub intra_split_flag: bool,
    /// `residual_adaptive_colour_transform_enabled_flag`.
    pub residual_adaptive_colour_transform_enabled_flag: bool,
    /// `cross_component_prediction_enabled_flag`.
    pub cross_component_prediction_enabled_flag: bool,
    /// `transform_skip_enabled_flag`.
    pub transform_skip_enabled_flag: bool,
    /// `Log2MaxTransformSkipSize`.
    pub log2_max_transform_skip_size: u8,
    /// `explicit_rdpcm_enabled_flag`.
    pub explicit_rdpcm_enabled_flag: bool,
    /// `implicit_rdpcm_enabled_flag`.
    pub implicit_rdpcm_enabled_flag: bool,
    /// Intra luma prediction mode used by implicit RDPCM inference.
    pub intra_luma_pred_mode: u8,
    /// `sign_data_hiding_enabled_flag`.
    pub sign_data_hiding_enabled_flag: bool,
    /// `cu_transquant_bypass_flag`.
    pub cu_transquant_bypass_flag: bool,
    /// Residual scan index: 0 diagonal, 1 horizontal, 2 vertical.
    pub scan_idx: u8,
}

/// Parsed residual syntax for one transform block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualCodingSyntax {
    /// `transform_skip_flag`, when present.
    pub transform_skip_flag: Option<bool>,
    /// `explicit_rdpcm_flag`, when present.
    pub explicit_rdpcm_flag: Option<bool>,
    /// `explicit_rdpcm_dir_flag`, when explicit RDPCM is enabled.
    pub explicit_rdpcm_dir_flag: Option<bool>,
    /// Last-significant-coefficient prefix values.
    pub last_sig_coeff_x_prefix: u64,
    /// Last-significant-coefficient y prefix value.
    pub last_sig_coeff_y_prefix: u64,
    /// Last-significant-coefficient suffix values, when their prefixes exceed
    /// three.
    pub last_sig_coeff_x_suffix: Option<u64>,
    /// Y suffix, when its prefix exceeds three.
    pub last_sig_coeff_y_suffix: Option<u64>,
    /// `coded_sub_block_flag` values in reverse residual scan order.
    pub coded_sub_block_flags: Vec<bool>,
    /// Significance flags in reverse coefficient scan order.
    pub sig_coeff_flags: Vec<bool>,
    /// Greater-than-one coefficient flags, in the same order as significant
    /// coefficients.
    pub coeff_abs_level_greater1_flags: Vec<bool>,
    /// The one possible greater-than-two flag.
    pub coeff_abs_level_greater2_flag: Option<bool>,
    /// Coefficient sign flags.
    pub coeff_sign_flags: Vec<bool>,
    /// Remaining coefficient magnitude syntax values.
    pub coeff_abs_level_remaining: Vec<u64>,
    /// Reconstructed signed coefficient levels in syntax order.
    pub coefficients: Vec<i64>,
}

/// Parsed transform-unit syntax and its component residual blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformUnitSyntax {
    /// `tu_residual_act_flag`, when ACT is enabled for the unit.
    pub residual_act_flag: Option<bool>,
    /// State after parsing `delta_qp()`.
    pub delta_qp: DeltaQpState,
    /// State after parsing `chroma_qp_offset()`.
    pub chroma_qp_offset: ChromaQpOffsetState,
    /// Luma residual, when `cbf_luma` is set.
    pub luma: Option<ResidualCodingSyntax>,
    /// Cb residual blocks.
    pub cb: Vec<ResidualCodingSyntax>,
    /// Cr residual blocks.
    pub cr: Vec<ResidualCodingSyntax>,
    /// Cross-component prediction syntax for Cb and Cr.
    pub cross_component_prediction: Vec<CrossComponentPredictionSyntax>,
}

/// One node of the recursive `transform_tree()` syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformTreeNode {
    /// Luma-space node origin.
    pub x: u64,
    /// Luma-space node origin.
    pub y: u64,
    /// Coding-unit base x coordinate.
    pub x_base: u64,
    /// Coding-unit base y coordinate.
    pub y_base: u64,
    /// Transform-block log2 size.
    pub log2_trafo_size: u8,
    /// Transform-tree depth.
    pub trafo_depth: u32,
    /// Child block index.
    pub blk_idx: u8,
    /// Whether the transform is split into four children.
    pub split_transform_flag: bool,
    /// Cb CBF values; the second entry is used by 4:2:2.
    pub cbf_cb: [Option<bool>; 2],
    /// Cr CBF values; the second entry is used by 4:2:2.
    pub cbf_cr: [Option<bool>; 2],
    /// Luma CBF at this transform depth.
    pub cbf_luma: Option<bool>,
    /// Child transform nodes.
    pub children: Vec<TransformTreeNode>,
    /// Leaf transform-unit syntax.
    pub transform_unit: Option<TransformUnitSyntax>,
}

/// Context for residual coefficient syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidualCodingContext {
    /// Transform-block log2 size.
    pub log2_trafo_size: u8,
    /// Whether the current CU is intra predicted.
    pub cu_pred_mode_intra: bool,
    /// `transform_skip_enabled_flag`.
    pub transform_skip_enabled_flag: bool,
    /// `Log2MaxTransformSkipSize`.
    pub log2_max_transform_skip_size: u8,
    /// `explicit_rdpcm_enabled_flag`.
    pub explicit_rdpcm_enabled_flag: bool,
    /// `implicit_rdpcm_enabled_flag`.
    pub implicit_rdpcm_enabled_flag: bool,
    /// Intra luma prediction mode.
    pub intra_luma_pred_mode: u8,
    /// `sign_data_hiding_enabled_flag`.
    pub sign_data_hiding_enabled_flag: bool,
    /// `cu_transquant_bypass_flag`.
    pub cu_transquant_bypass_flag: bool,
    /// Residual scan index: 0 diagonal, 1 horizontal, 2 vertical.
    pub scan_idx: u8,
}

/// Clause 9 residual-coding options signalled by range/SCC extensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidualCodingOptions {
    /// `transform_skip_context_enabled_flag`.
    pub transform_skip_context_enabled_flag: bool,
    /// `extended_precision_processing_flag`.
    pub extended_precision_processing_flag: bool,
    /// `persistent_rice_adaptation_enabled_flag`.
    pub persistent_rice_adaptation_enabled_flag: bool,
    /// `cabac_bypass_alignment_enabled_flag`.
    pub cabac_bypass_alignment_enabled_flag: bool,
    /// Luma bit depth used by limited EGk.
    pub bit_depth_luma: u8,
    /// Chroma bit depth used by limited EGk.
    pub bit_depth_chroma: u8,
}

impl ResidualCodingOptions {
    fn from_context(context: ResidualCodingContext) -> Self {
        Self {
            transform_skip_context_enabled_flag: context.transform_skip_enabled_flag,
            extended_precision_processing_flag: false,
            persistent_rice_adaptation_enabled_flag: false,
            cabac_bypass_alignment_enabled_flag: false,
            bit_depth_luma: 8,
            bit_depth_chroma: 8,
        }
    }

    fn from_transform_context(context: TransformTreeContext) -> Self {
        Self {
            transform_skip_context_enabled_flag: context.transform_skip_enabled_flag,
            extended_precision_processing_flag: false,
            persistent_rice_adaptation_enabled_flag: false,
            cabac_bypass_alignment_enabled_flag: false,
            bit_depth_luma: 8,
            bit_depth_chroma: 8,
        }
    }
}

/// Stateful `StatCoeff[4]` values from Clause 9.3.2.1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualRiceState {
    stat_coeff: [u8; 4],
}

impl ResidualRiceState {
    /// Creates the state used at the start of a CTU or independent slice.
    #[must_use]
    pub const fn new() -> Self {
        Self { stat_coeff: [0; 4] }
    }

    /// Returns the four current `StatCoeff` values.
    #[must_use]
    pub const fn stat_coeff(&self) -> [u8; 4] {
        self.stat_coeff
    }

    /// Replaces the four current `StatCoeff` values after WPP/dependent-slice
    /// synchronization.
    pub fn set_stat_coeff(&mut self, stat_coeff: [u8; 4]) {
        self.stat_coeff = stat_coeff;
    }

    fn initial_rice_parameter(
        &self,
        c_idx: u8,
        transform_skip_or_bypass: bool,
        persistent: bool,
    ) -> u8 {
        if !persistent {
            return 0;
        }
        let sb_type = 2 * usize::from(c_idx == 0) + usize::from(transform_skip_or_bypass);
        self.stat_coeff[sb_type] / 4
    }

    fn update_first_remaining(&mut self, sb_type: usize, value: u64) {
        let stat = self.stat_coeff[sb_type] / 4;
        if value >= 3 * (1_u64 << stat) {
            self.stat_coeff[sb_type] = self.stat_coeff[sb_type].saturating_add(1);
        } else if 2 * value < (1_u64 << stat) && self.stat_coeff[sb_type] > 0 {
            self.stat_coeff[sb_type] = self.stat_coeff[sb_type].saturating_sub(1);
        }
    }
}

impl Default for ResidualRiceState {
    fn default() -> Self {
        Self::new()
    }
}

fn residual_scan_order(size: usize, scan_idx: u8) -> Result<Vec<(usize, usize)>, SyntaxError> {
    let size = u32::try_from(size)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("residual scan size is too large"))?;
    let coordinates = match scan_idx {
        0 => crate::up_right_diagonal_scan(size),
        1 => crate::horizontal_scan(size),
        2 => crate::vertical_scan(size),
        _ => {
            return Err(SyntaxError::InvalidSyntaxValue(
                "residual scan index must be zero, one, or two",
            ));
        }
    }
    .map_err(|_| SyntaxError::InvalidSyntaxValue("invalid residual scan size"))?;
    Ok(coordinates
        .into_iter()
        .map(|(x, y)| (x as usize, y as usize))
        .collect())
}

fn decode_last_coeff_coordinate(
    prefix: u64,
    suffix: Option<u64>,
    log2_trafo_size: u8,
) -> Result<usize, SyntaxError> {
    let size = 1u64
        .checked_shl(u32::from(log2_trafo_size))
        .ok_or(SyntaxError::InvalidSyntaxValue("transform size overflows"))?;
    let coordinate = if prefix <= 3 {
        prefix
    } else {
        let shift = prefix / 2 - 1;
        let suffix_bits = usize::try_from(shift)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("coefficient suffix is too large"))?;
        let suffix = suffix.ok_or(SyntaxError::InvalidSyntaxValue(
            "coefficient suffix is missing",
        ))?;
        if suffix_bits < 64 && suffix >= (1u64 << suffix_bits) {
            return Err(SyntaxError::InvalidSyntaxValue(
                "coefficient suffix exceeds its coded width",
            ));
        }
        let base = 1u64
            .checked_shl(u32::try_from(shift).unwrap_or(u32::MAX))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "coefficient prefix overflows",
            ))?;
        base.checked_mul(2 + prefix % 2)
            .and_then(|value| value.checked_add(suffix))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "last coefficient coordinate overflows",
            ))?
    };
    if coordinate >= size {
        return Err(SyntaxError::InvalidSyntaxValue(
            "last coefficient coordinate exceeds transform block",
        ));
    }
    usize::try_from(coordinate)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("last coefficient coordinate is too large"))
}

fn read_last_sig_coeff_prefix(
    cabac: &mut impl CabacReader,
    table: CabacContextTable,
    c_idx: u8,
    log2_trafo_size: u8,
) -> Result<u64, SyntaxError> {
    if !(2..=6).contains(&log2_trafo_size) {
        return Err(SyntaxError::InvalidSyntaxValue(
            "last coefficient transform size is out of range",
        ));
    }
    let c_max = (u64::from(log2_trafo_size) << 1) - 1;
    let (ctx_offset, ctx_shift) = if c_idx == 0 {
        (
            3 * (usize::from(log2_trafo_size) - 2) + ((usize::from(log2_trafo_size) - 1) >> 2),
            (usize::from(log2_trafo_size) + 1) >> 2,
        )
    } else {
        (15, usize::from(log2_trafo_size) - 2)
    };
    let mut prefix = 0_u64;
    while prefix < c_max {
        let ctx_inc = ctx_offset + (prefix as usize >> ctx_shift);
        if cabac.read_ae_named(table, ctx_inc)? == 0 {
            break;
        }
        prefix += 1;
    }
    Ok(prefix)
}

/// Parses `residual_coding()` and preserves all CABAC-coded syntax fields.
pub fn parse_residual_coding(
    cabac: &mut impl CabacReader,
    context: ResidualCodingContext,
) -> Result<ResidualCodingSyntax, SyntaxError> {
    parse_residual_coding_for_component(cabac, context, 0)
}

/// Parses `residual_coding()` with the component index used by Clause 9
/// context derivation (`0` luma, `1` Cb, `2` Cr).
pub fn parse_residual_coding_for_component(
    cabac: &mut impl CabacReader,
    context: ResidualCodingContext,
    c_idx: u8,
) -> Result<ResidualCodingSyntax, SyntaxError> {
    parse_residual_coding_for_component_with_options(
        cabac,
        context,
        c_idx,
        ResidualCodingOptions::from_context(context),
    )
}

/// Parses residual coding with the optional range/SCC Clause 9 controls.
pub fn parse_residual_coding_for_component_with_options(
    cabac: &mut impl CabacReader,
    context: ResidualCodingContext,
    c_idx: u8,
    options: ResidualCodingOptions,
) -> Result<ResidualCodingSyntax, SyntaxError> {
    let mut rice_state = ResidualRiceState::new();
    parse_residual_coding_for_component_with_options_and_state(
        cabac,
        context,
        c_idx,
        options,
        &mut rice_state,
    )
}

/// Parses residual coding while preserving `StatCoeff` across transform
/// blocks in the same slice/CTU state.
pub fn parse_residual_coding_for_component_with_options_and_state(
    cabac: &mut impl CabacReader,
    context: ResidualCodingContext,
    c_idx: u8,
    options: ResidualCodingOptions,
    rice_state: &mut ResidualRiceState,
) -> Result<ResidualCodingSyntax, SyntaxError> {
    if c_idx > 2 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "residual component index must be 0, 1, or 2",
        ));
    }
    if context.log2_trafo_size < 2 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "residual transform size must be at least 4x4",
        ));
    }
    let transform_skip_flag = if context.transform_skip_enabled_flag
        && !context.cu_transquant_bypass_flag
        && context.log2_trafo_size <= context.log2_max_transform_skip_size
    {
        Some(
            cabac.read_ae_named(
                CabacContextTable::TransformSkip,
                if c_idx == 0 { 0 } else { 3 },
            )? != 0,
        )
    } else {
        None
    };
    let rdpcm_active = transform_skip_flag == Some(true) || context.cu_transquant_bypass_flag;
    let explicit_rdpcm_flag =
        if !context.cu_pred_mode_intra && context.explicit_rdpcm_enabled_flag && rdpcm_active {
            Some(
                cabac.read_ae_named(
                    CabacContextTable::ExplicitRdpcm,
                    if c_idx == 0 { 0 } else { 2 },
                )? != 0,
            )
        } else {
            None
        };
    let explicit_rdpcm_dir_flag = if explicit_rdpcm_flag == Some(true) {
        Some(
            cabac.read_ae_named(
                CabacContextTable::ExplicitRdpcmDir,
                if c_idx == 0 { 0 } else { 2 },
            )? != 0,
        )
    } else {
        None
    };
    let last_sig_coeff_x_prefix = read_last_sig_coeff_prefix(
        cabac,
        CabacContextTable::LastSigCoeffXPrefix,
        c_idx,
        context.log2_trafo_size,
    )?;
    let last_sig_coeff_y_prefix = read_last_sig_coeff_prefix(
        cabac,
        CabacContextTable::LastSigCoeffYPrefix,
        c_idx,
        context.log2_trafo_size,
    )?;
    let last_sig_coeff_x_suffix = if last_sig_coeff_x_prefix > 3 {
        Some(cabac.read_fixed_bypass((1_u64 << ((last_sig_coeff_x_prefix >> 1) - 1)) - 1)?)
    } else {
        None
    };
    let last_sig_coeff_y_suffix = if last_sig_coeff_y_prefix > 3 {
        Some(cabac.read_fixed_bypass((1_u64 << ((last_sig_coeff_y_prefix >> 1) - 1)) - 1)?)
    } else {
        None
    };

    let decoded_last_x = decode_last_coeff_coordinate(
        last_sig_coeff_x_prefix,
        last_sig_coeff_x_suffix,
        context.log2_trafo_size,
    )?;
    let decoded_last_y = decode_last_coeff_coordinate(
        last_sig_coeff_y_prefix,
        last_sig_coeff_y_suffix,
        context.log2_trafo_size,
    )?;
    let (last_x, last_y) = if context.scan_idx == 2 {
        (decoded_last_y, decoded_last_x)
    } else {
        (decoded_last_x, decoded_last_y)
    };
    let transform_side = 1usize
        .checked_shl(u32::from(context.log2_trafo_size))
        .ok_or(SyntaxError::InvalidSyntaxValue("transform size overflows"))?;
    let coefficient_scan_order = residual_scan_order(4, context.scan_idx)?;
    let sub_block_side = 1usize
        .checked_shl(u32::from(context.log2_trafo_size - 2))
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "residual block count overflows",
        ))?;
    let sub_block_scan_order = residual_scan_order(sub_block_side, context.scan_idx)?;
    let last_sub_block = sub_block_scan_order
        .iter()
        .position(|&(x, y)| x == last_x / 4 && y == last_y / 4)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "last coefficient sub-block is not in scan order",
        ))?;
    let last_scan_pos = coefficient_scan_order
        .iter()
        .position(|&(x, y)| x == last_x % 4 && y == last_y % 4)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "last coefficient is not in scan order",
        ))?;
    if transform_side < 4 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "transform side must be at least 4",
        ));
    }
    let mut coded_sub_block_grid = vec![vec![false; sub_block_side]; sub_block_side];
    let mut coded_sub_block_flags = Vec::with_capacity(last_sub_block + 1);
    for index in (0..=last_sub_block).rev() {
        let (x_s, y_s) = sub_block_scan_order[index];
        let mut csbf_ctx = 0_usize;
        if x_s + 1 < sub_block_side {
            csbf_ctx += usize::from(coded_sub_block_grid[y_s][x_s + 1]);
        }
        if y_s + 1 < sub_block_side {
            csbf_ctx += usize::from(coded_sub_block_grid[y_s + 1][x_s]);
        }
        let coded = if index > 0 && index < last_sub_block {
            cabac.read_ae_named(
                CabacContextTable::CodedSubBlock,
                if c_idx == 0 {
                    csbf_ctx.min(1)
                } else {
                    2 + csbf_ctx.min(1)
                },
            )? != 0
        } else {
            true
        };
        coded_sub_block_grid[y_s][x_s] = coded;
        coded_sub_block_flags.push(coded);
    }

    let mut block_sig_coeff_flags = Vec::with_capacity(coded_sub_block_flags.len());
    for (block_index, coded) in coded_sub_block_flags.iter().copied().enumerate() {
        let sub_block_index = last_sub_block - block_index;
        let mut block_flags = vec![false; 16];
        let is_last_sub_block = sub_block_index == last_sub_block;
        if is_last_sub_block {
            block_flags[last_scan_pos] = true;
        }
        let first_scan_pos = if is_last_sub_block {
            last_scan_pos.saturating_sub(1)
        } else {
            15
        };
        let infer_dc_sig_coeff_flag = sub_block_index > 0 && sub_block_index < last_sub_block;
        let mut infer_dc = infer_dc_sig_coeff_flag;
        for coefficient in (0..=first_scan_pos).rev() {
            if is_last_sub_block && coefficient == last_scan_pos {
                continue;
            }
            let (x_c, y_c) = coefficient_scan_order[coefficient];
            let x_s = x_c >> 2;
            let y_s = y_c >> 2;
            let sig_ctx = if options.transform_skip_context_enabled_flag
                && (transform_skip_flag == Some(true) || context.cu_transquant_bypass_flag)
            {
                if c_idx == 0 {
                    42
                } else {
                    16
                }
            } else if context.log2_trafo_size == 2 {
                let base = [0, 1, 4, 5, 2, 3, 4, 5, 6, 6, 8, 8, 7, 7, 8][y_c * 4 + x_c];
                if c_idx == 0 {
                    base
                } else {
                    27 + base
                }
            } else if x_c + y_c == 0 {
                0
            } else {
                let mut prev_csbf = 0_usize;
                if x_s + 1 < sub_block_side {
                    prev_csbf += usize::from(coded_sub_block_grid[y_s][x_s + 1]);
                }
                if y_s + 1 < sub_block_side {
                    prev_csbf += usize::from(coded_sub_block_grid[y_s + 1][x_s]) << 1;
                }
                let x_p = x_c & 3;
                let y_p = y_c & 3;
                let mut sig_ctx = match prev_csbf {
                    0 => usize::from(x_p + y_p < 3) + usize::from(x_p + y_p == 0),
                    1 => usize::from(y_p == 1) + usize::from(y_p == 0),
                    2 => usize::from(x_p == 1) + usize::from(x_p == 0),
                    _ => 2,
                };
                if c_idx == 0 {
                    if x_s + y_s > 0 {
                        sig_ctx += 3;
                    }
                    sig_ctx += if context.log2_trafo_size == 3 {
                        if context.scan_idx == 0 {
                            9
                        } else {
                            15
                        }
                    } else {
                        21
                    };
                    sig_ctx
                } else {
                    sig_ctx += if context.log2_trafo_size == 3 { 9 } else { 12 };
                    27 + sig_ctx
                }
            };
            let significant = if !coded {
                false
            } else if coefficient > 0 || !infer_dc {
                cabac.read_ae_named(CabacContextTable::SigCoeff, sig_ctx)? != 0
            } else {
                true
            };
            block_flags[coefficient] = significant;
            if significant {
                infer_dc = false;
            }
        }
        block_sig_coeff_flags.push(block_flags);
    }
    let mut sig_coeff_flags = Vec::with_capacity(coded_sub_block_flags.len() * 16);
    for (block_index, block_flags) in block_sig_coeff_flags.iter().enumerate() {
        let first_scan_pos = if last_sub_block - block_index == last_sub_block {
            last_scan_pos
        } else {
            15
        };
        for coefficient in (0..=first_scan_pos).rev() {
            sig_coeff_flags.push(block_flags[coefficient]);
        }
    }
    let significant_count = sig_coeff_flags.iter().filter(|flag| **flag).count();
    let mut coeff_abs_level_greater1_flags = Vec::with_capacity(significant_count.min(8));
    let mut greater1_context_sets = Vec::with_capacity(significant_count.min(8));
    let mut significant_seen = 0usize;
    let mut ctx_set = 0_usize;
    let mut greater1_ctx = 0_usize;
    let mut last_greater1_flag = false;
    let mut first_sub_block = true;
    for (block_index, block_flags) in block_sig_coeff_flags.iter().enumerate() {
        let sub_block_index = last_sub_block - block_index;
        let mut first_in_sub_block = true;
        let first_scan_pos = if sub_block_index == last_sub_block {
            last_scan_pos
        } else {
            15
        };
        for coefficient in (0..=first_scan_pos).rev() {
            let significant = block_flags[coefficient];
            if significant {
                if first_in_sub_block {
                    ctx_set = if sub_block_index == 0 || c_idx > 0 {
                        0
                    } else {
                        2
                    };
                    if !first_sub_block {
                        if greater1_ctx > 0 {
                            greater1_ctx = if last_greater1_flag {
                                0
                            } else {
                                greater1_ctx + 1
                            };
                        }
                        if greater1_ctx == 0 {
                            ctx_set += 1;
                        }
                    }
                    greater1_ctx = 1;
                    first_in_sub_block = false;
                    first_sub_block = false;
                } else if greater1_ctx > 0 {
                    greater1_ctx = if last_greater1_flag {
                        0
                    } else {
                        greater1_ctx + 1
                    };
                }
                if significant_seen < 8 {
                    let mut ctx_inc = (ctx_set * 4) + greater1_ctx.min(3);
                    if c_idx > 0 {
                        ctx_inc += 16;
                    }
                    greater1_context_sets.push(ctx_set);
                    coeff_abs_level_greater1_flags.push(
                        cabac.read_ae_named(CabacContextTable::CoeffAbsLevelGreater1, ctx_inc)?
                            != 0,
                    );
                    last_greater1_flag = *coeff_abs_level_greater1_flags.last().ok_or(
                        SyntaxError::InvalidSyntaxValue("greater1 context state is empty"),
                    )?;
                }
                significant_seen += 1;
            }
        }
    }
    let last_greater1 = coeff_abs_level_greater1_flags.iter().position(|flag| *flag);
    let coeff_abs_level_greater2_flag = if last_greater1.is_some() {
        let ctx_set = last_greater1
            .and_then(|index| greater1_context_sets.get(index).copied())
            .unwrap_or(0);
        Some(
            cabac.read_ae_named(
                CabacContextTable::CoeffAbsLevelGreater2,
                ctx_set + if c_idx > 0 { 4 } else { 0 },
            )? != 0,
        )
    } else {
        None
    };
    if options.cabac_bypass_alignment_enabled_flag {
        cabac.cabac_bypass_alignment()?;
    }
    let implicit_rdpcm_sign_hiding_disabled = context.cu_pred_mode_intra
        && context.implicit_rdpcm_enabled_flag
        && transform_skip_flag == Some(true)
        && (context.intra_luma_pred_mode == 10 || context.intra_luma_pred_mode == 26);
    let sign_hiding_allowed = context.sign_data_hiding_enabled_flag
        && !context.cu_transquant_bypass_flag
        && !implicit_rdpcm_sign_hiding_disabled
        && explicit_rdpcm_flag != Some(true);
    let sign_hidden_by_block: Vec<Option<usize>> = block_sig_coeff_flags
        .iter()
        .enumerate()
        .map(|(block_index, block_flags)| {
            let sub_block_index = last_sub_block - block_index;
            let first_scan_pos = if sub_block_index == last_sub_block {
                last_scan_pos
            } else {
                15
            };
            let significant_positions: Vec<usize> = (0..=first_scan_pos)
                .filter(|&position| block_flags[position])
                .collect();
            match (significant_positions.first(), significant_positions.last()) {
                (Some(&first), Some(&last))
                    if sign_hiding_allowed && last.saturating_sub(first) > 3 =>
                {
                    Some(first)
                }
                _ => None,
            }
        })
        .collect();
    let mut coeff_sign_flags = Vec::with_capacity(significant_count);
    for (block_index, block_flags) in block_sig_coeff_flags.iter().enumerate() {
        let sub_block_index = last_sub_block - block_index;
        let first_scan_pos = if sub_block_index == last_sub_block {
            last_scan_pos
        } else {
            15
        };
        for coefficient in (0..=first_scan_pos).rev() {
            if block_flags[coefficient] && sign_hidden_by_block[block_index] != Some(coefficient) {
                coeff_sign_flags.push(cabac.read_bypass_bin()? != 0);
            }
        }
    }
    let mut coeff_abs_level_remaining = Vec::new();
    let mut coefficients = Vec::with_capacity(significant_count);
    let mut greater1_index = 0usize;
    let mut sign_index = 0usize;
    for (block_index, block_flags) in block_sig_coeff_flags.iter().enumerate() {
        let sub_block_index = last_sub_block - block_index;
        let first_scan_pos = if sub_block_index == last_sub_block {
            last_scan_pos
        } else {
            15
        };
        let transform_skip_or_bypass =
            transform_skip_flag == Some(true) || context.cu_transquant_bypass_flag;
        let sb_type = 2 * usize::from(c_idx == 0) + usize::from(transform_skip_or_bypass);
        let mut rice_parameter = rice_state.initial_rice_parameter(
            c_idx,
            transform_skip_or_bypass,
            options.persistent_rice_adaptation_enabled_flag,
        );
        let rice_parameter_max = if options.persistent_rice_adaptation_enabled_flag {
            8
        } else {
            4
        };
        let mut first_remaining = true;
        let mut sum_abs_level = 0i64;
        for coefficient in (0..=first_scan_pos).rev() {
            if !block_flags[coefficient] {
                continue;
            }
            let greater1 = coeff_abs_level_greater1_flags
                .get(greater1_index)
                .copied()
                .unwrap_or(false);
            let greater2 = if Some(greater1_index) == last_greater1 {
                coeff_abs_level_greater2_flag.unwrap_or(false)
            } else {
                false
            };
            let base_level = 1 + u64::from(greater1) + u64::from(greater2);
            let threshold = if greater1_index < 8 {
                if Some(greater1_index) == last_greater1 {
                    3
                } else {
                    2
                }
            } else {
                1
            };
            let remaining = if base_level == threshold {
                let derived_rice = rice_parameter.min(rice_parameter_max);
                let value = cabac.read_coeff_abs_level_remaining_with_options(
                    base_level,
                    derived_rice,
                    options.extended_precision_processing_flag,
                    if c_idx == 0 {
                        options.bit_depth_luma.saturating_add(6).max(15)
                    } else {
                        options.bit_depth_chroma.saturating_add(6).max(15)
                    },
                )?;
                coeff_abs_level_remaining.push(value);
                if options.persistent_rice_adaptation_enabled_flag && first_remaining {
                    rice_state.update_first_remaining(sb_type, value);
                    first_remaining = false;
                }
                let current_abs_level = base_level
                    .checked_add(value)
                    .ok_or(SyntaxError::ExpGolombOverflow)?;
                if current_abs_level > 3 * (1_u64 << derived_rice) {
                    rice_parameter = rice_parameter.saturating_add(1).min(rice_parameter_max);
                }
                value
            } else {
                0
            };
            let magnitude = i64::try_from(remaining + base_level).map_err(|_| {
                SyntaxError::InvalidSyntaxValue("coefficient magnitude is too large")
            })?;
            sum_abs_level += magnitude;
            let sign = if sign_hidden_by_block[block_index] == Some(coefficient) {
                sum_abs_level % 2 != 0
            } else {
                let value = coeff_sign_flags.get(sign_index).copied().unwrap_or(false);
                sign_index += 1;
                value
            };
            coefficients.push(if sign { -magnitude } else { magnitude });
            greater1_index += 1;
        }
    }
    Ok(ResidualCodingSyntax {
        transform_skip_flag,
        explicit_rdpcm_flag,
        explicit_rdpcm_dir_flag,
        last_sig_coeff_x_prefix,
        last_sig_coeff_y_prefix,
        last_sig_coeff_x_suffix,
        last_sig_coeff_y_suffix,
        coded_sub_block_flags,
        sig_coeff_flags,
        coeff_abs_level_greater1_flags,
        coeff_abs_level_greater2_flag,
        coeff_sign_flags,
        coeff_abs_level_remaining,
        coefficients,
    })
}

fn residual_context(context: TransformTreeContext, log2_trafo_size: u8) -> ResidualCodingContext {
    ResidualCodingContext {
        log2_trafo_size,
        cu_pred_mode_intra: context.cu_pred_mode_intra,
        transform_skip_enabled_flag: context.transform_skip_enabled_flag,
        log2_max_transform_skip_size: context.log2_max_transform_skip_size,
        explicit_rdpcm_enabled_flag: context.explicit_rdpcm_enabled_flag,
        implicit_rdpcm_enabled_flag: context.implicit_rdpcm_enabled_flag,
        intra_luma_pred_mode: context.intra_luma_pred_mode,
        sign_data_hiding_enabled_flag: context.sign_data_hiding_enabled_flag,
        cu_transquant_bypass_flag: context.cu_transquant_bypass_flag,
        scan_idx: context.scan_idx,
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_transform_unit(
    cabac: &mut impl CabacReader,
    context: TransformTreeContext,
    log2_trafo_size: u8,
    cbf_luma: bool,
    cbf_cb: &[Option<bool>; 2],
    cbf_cr: &[Option<bool>; 2],
    delta_qp: &mut DeltaQpState,
    chroma_qp_offset: &mut ChromaQpOffsetState,
    residual_options: ResidualCodingOptions,
    rice_state: &mut ResidualRiceState,
) -> Result<TransformUnitSyntax, SyntaxError> {
    let has_chroma =
        cbf_cb.iter().flatten().any(|flag| *flag) || cbf_cr.iter().flatten().any(|flag| *flag);
    let residual_act_flag = if context.residual_adaptive_colour_transform_enabled_flag {
        Some(cabac.read_ae_named(CabacContextTable::TuResidualAct, 0)? != 0)
    } else {
        None
    };
    delta_qp.parse(cabac, true)?;
    if has_chroma && !context.cu_transquant_bypass_flag {
        chroma_qp_offset.parse(cabac, true, 0)?;
    }
    let residual_context = residual_context(context, log2_trafo_size);
    let luma = if cbf_luma {
        Some(parse_residual_coding_for_component_with_options_and_state(
            cabac,
            residual_context,
            0,
            residual_options,
            rice_state,
        )?)
    } else {
        None
    };
    let mut cross_component_prediction = Vec::new();
    let mut cb = Vec::new();
    let mut cr = Vec::new();
    if context.cross_component_prediction_enabled_flag && cbf_luma && has_chroma {
        cross_component_prediction.push(parse_cross_component_prediction(cabac)?);
    }
    for flag in cbf_cb.iter().flatten() {
        if *flag {
            cb.push(parse_residual_coding_for_component_with_options_and_state(
                cabac,
                residual_context,
                1,
                residual_options,
                rice_state,
            )?);
        }
    }
    if context.cross_component_prediction_enabled_flag && cbf_luma && has_chroma {
        cross_component_prediction.push(parse_cross_component_prediction(cabac)?);
    }
    for flag in cbf_cr.iter().flatten() {
        if *flag {
            cr.push(parse_residual_coding_for_component_with_options_and_state(
                cabac,
                residual_context,
                2,
                residual_options,
                rice_state,
            )?);
        }
    }
    Ok(TransformUnitSyntax {
        residual_act_flag,
        delta_qp: *delta_qp,
        chroma_qp_offset: *chroma_qp_offset,
        luma,
        cb,
        cr,
        cross_component_prediction,
    })
}

#[allow(clippy::too_many_arguments)]
fn parse_transform_tree_node(
    cabac: &mut impl CabacReader,
    context: TransformTreeContext,
    x: u64,
    y: u64,
    x_base: u64,
    y_base: u64,
    log2_trafo_size: u8,
    trafo_depth: u32,
    blk_idx: u8,
    base_cbf_cb: bool,
    base_cbf_cr: bool,
    delta_qp: &mut DeltaQpState,
    chroma_qp_offset: &mut ChromaQpOffsetState,
    residual_options: ResidualCodingOptions,
    rice_state: &mut ResidualRiceState,
) -> Result<TransformTreeNode, SyntaxError> {
    let split_allowed = log2_trafo_size <= context.max_tb_log2_size
        && log2_trafo_size > context.min_tb_log2_size
        && trafo_depth < context.max_trafo_depth
        && !(context.intra_split_flag && trafo_depth == 0);
    let split_transform_flag = if split_allowed {
        let ctx_inc = usize::from(5_u8.saturating_sub(log2_trafo_size));
        cabac.read_ae_named(CabacContextTable::SplitTransform, ctx_inc)? != 0
    } else {
        false
    };
    let has_chroma_cbf =
        (log2_trafo_size > 2 && context.chroma_array_type != 0) || context.chroma_array_type == 3;
    let mut cbf_cb = [None; 2];
    let mut cbf_cr = [None; 2];
    if has_chroma_cbf && (trafo_depth == 0 || base_cbf_cb) {
        cbf_cb[0] =
            Some(cabac.read_ae_named(CabacContextTable::CbfChroma, trafo_depth as usize)? != 0);
        if context.chroma_array_type == 2 && (!split_transform_flag || log2_trafo_size == 3) {
            cbf_cb[1] = Some(
                cabac.read_ae_named(CabacContextTable::CbfChroma, 12 + trafo_depth as usize)? != 0,
            );
        }
    }
    if has_chroma_cbf && (trafo_depth == 0 || base_cbf_cr) {
        cbf_cr[0] =
            Some(cabac.read_ae_named(CabacContextTable::CbfChroma, trafo_depth as usize)? != 0);
        if context.chroma_array_type == 2 && (!split_transform_flag || log2_trafo_size == 3) {
            cbf_cr[1] = Some(
                cabac.read_ae_named(CabacContextTable::CbfChroma, 12 + trafo_depth as usize)? != 0,
            );
        }
    }
    let mut children = Vec::new();
    let mut transform_unit = None;
    let cbf_chroma =
        cbf_cb.iter().flatten().any(|flag| *flag) || cbf_cr.iter().flatten().any(|flag| *flag);
    let cbf_luma = if split_transform_flag {
        None
    } else if context.cu_pred_mode_intra || trafo_depth != 0 || cbf_chroma {
        Some(cabac.read_ae_named(CabacContextTable::CbfLuma, usize::from(trafo_depth != 0))? != 0)
    } else {
        None
    };
    if split_transform_flag {
        let child_size = 1u64.checked_shl(u32::from(log2_trafo_size - 1)).ok_or(
            SyntaxError::InvalidSyntaxValue("transform child size overflows"),
        )?;
        let child_log2 = log2_trafo_size
            .checked_sub(1)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "transform depth underflows",
            ))?;
        for (index, (child_x, child_y)) in [
            (x, y),
            (x + child_size, y),
            (x, y + child_size),
            (x + child_size, y + child_size),
        ]
        .into_iter()
        .enumerate()
        {
            children.push(parse_transform_tree_node(
                cabac,
                context,
                child_x,
                child_y,
                x_base,
                y_base,
                child_log2,
                trafo_depth + 1,
                index as u8,
                cbf_cb[0].unwrap_or(false),
                cbf_cr[0].unwrap_or(false),
                delta_qp,
                chroma_qp_offset,
                residual_options,
                rice_state,
            )?);
        }
    } else if cbf_luma == Some(true) || cbf_chroma {
        transform_unit = Some(parse_transform_unit(
            cabac,
            context,
            log2_trafo_size,
            cbf_luma == Some(true),
            &cbf_cb,
            &cbf_cr,
            delta_qp,
            chroma_qp_offset,
            residual_options,
            rice_state,
        )?);
    }
    Ok(TransformTreeNode {
        x,
        y,
        x_base,
        y_base,
        log2_trafo_size,
        trafo_depth,
        blk_idx,
        split_transform_flag,
        cbf_cb,
        cbf_cr,
        cbf_luma,
        children,
        transform_unit,
    })
}

/// Parses the complete recursive `transform_tree()` syntax.
pub fn parse_transform_tree(
    cabac: &mut impl CabacReader,
    context: TransformTreeContext,
    x: u64,
    y: u64,
    log2_trafo_size: u8,
) -> Result<TransformTreeNode, SyntaxError> {
    parse_transform_tree_with_residual_options(
        cabac,
        context,
        x,
        y,
        log2_trafo_size,
        ResidualCodingOptions::from_transform_context(context),
    )
}

/// Parses `transform_tree()` with the optional Clause 9 range/SCC residual
/// controls and a Rice state shared by all transform units in the tree.
pub fn parse_transform_tree_with_residual_options(
    cabac: &mut impl CabacReader,
    context: TransformTreeContext,
    x: u64,
    y: u64,
    log2_trafo_size: u8,
    residual_options: ResidualCodingOptions,
) -> Result<TransformTreeNode, SyntaxError> {
    let mut rice_state = ResidualRiceState::new();
    parse_transform_tree_with_residual_options_and_state(
        cabac,
        context,
        x,
        y,
        log2_trafo_size,
        residual_options,
        &mut rice_state,
    )
}

/// Parses `transform_tree()` while preserving the supplied Clause 9
/// `StatCoeff` state across transform trees in one slice or CTU sequence.
pub fn parse_transform_tree_with_residual_options_and_state(
    cabac: &mut impl CabacReader,
    context: TransformTreeContext,
    x: u64,
    y: u64,
    log2_trafo_size: u8,
    residual_options: ResidualCodingOptions,
    rice_state: &mut ResidualRiceState,
) -> Result<TransformTreeNode, SyntaxError> {
    let mut delta_qp = DeltaQpState::new();
    let mut chroma_qp_offset = ChromaQpOffsetState::new();
    parse_transform_tree_node(
        cabac,
        context,
        x,
        y,
        x,
        y,
        log2_trafo_size,
        0,
        0,
        false,
        false,
        &mut delta_qp,
        &mut chroma_qp_offset,
        residual_options,
        rice_state,
    )
}

/// Context for `palette_coding()`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaletteCodingContext {
    /// Coding-block width/height in luma samples (`nCbS`).
    pub n_cb_s: usize,
    /// Number of predictor entries available before the current CU.
    pub predictor_palette_size: usize,
    /// `palette_max_size`.
    pub palette_max_size: u64,
    /// `ChromaArrayType`.
    pub chroma_array_type: u8,
    /// `cu_qp_delta_enabled_flag`.
    pub cu_qp_delta_enabled_flag: bool,
    /// `cu_chroma_qp_offset_enabled_flag`.
    pub cu_chroma_qp_offset_enabled_flag: bool,
    /// `chroma_qp_offset_list_len_minus1`.
    pub chroma_qp_offset_list_len_minus1: u64,
    /// `cu_transquant_bypass_flag`.
    pub cu_transquant_bypass_flag: bool,
}

/// One palette index run in palette scan order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaletteRunSyntax {
    /// Starting scan position.
    pub scan_position: usize,
    /// `copy_above_palette_indices_flag`.
    pub copy_above_indices_flag: bool,
    /// Palette index selected by this run, when it is not copied above.
    pub palette_index: Option<u64>,
    /// `palette_run_prefix`, when the run does not reach the block end.
    pub run_prefix: Option<u64>,
    /// `palette_run_suffix`, when present.
    pub run_suffix: Option<u64>,
    /// Number of samples covered by this run.
    pub run_length: usize,
}

/// Parsed `palette_coding()` syntax from §7.3.8.13.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaletteCodingSyntax {
    /// Predictor entries reused by index.
    pub predictor_reuse_flags: Vec<bool>,
    /// Number of predicted entries reused by this CU.
    pub num_predicted_palette_entries: usize,
    /// `num_signalled_palette_entries`.
    pub num_signalled_palette_entries: usize,
    /// New palette entries, indexed by component then entry.
    pub new_palette_entries: Vec<Vec<u64>>,
    /// `palette_escape_val_present_flag`.
    pub palette_escape_val_present_flag: bool,
    /// `num_palette_indices_minus1`, when palette indices are coded.
    pub num_palette_indices_minus1: Option<usize>,
    /// `palette_idx_idc` values.
    pub palette_index_idc: Vec<u64>,
    /// `copy_above_indices_for_final_run_flag`.
    pub copy_above_indices_for_final_run_flag: bool,
    /// `palette_transpose_flag`.
    pub palette_transpose_flag: bool,
    /// Palette runs in scan order.
    pub runs: Vec<PaletteRunSyntax>,
    /// QP delta state after palette escape syntax.
    pub delta_qp: DeltaQpState,
    /// Chroma QP offset state after palette escape syntax.
    pub chroma_qp_offset: ChromaQpOffsetState,
    /// Escape values in component/scan order.
    pub escape_values: Vec<Vec<u64>>,
}

fn checked_usize(value: u64, message: &'static str) -> Result<usize, SyntaxError> {
    usize::try_from(value).map_err(|_| SyntaxError::InvalidSyntaxValue(message))
}

/// Parses `palette_coding()` including index runs and escape values.
pub fn parse_palette_coding(
    cabac: &mut impl CabacReader,
    context: PaletteCodingContext,
) -> Result<PaletteCodingSyntax, SyntaxError> {
    parse_palette_coding_with_bit_depth(cabac, context, 8, 8)
}

/// Parses `palette_coding()` with explicit luma and chroma bit depths.
pub fn parse_palette_coding_with_bit_depth(
    cabac: &mut impl CabacReader,
    context: PaletteCodingContext,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
) -> Result<PaletteCodingSyntax, SyntaxError> {
    if context.n_cb_s == 0 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "palette coding block size must be non-zero",
        ));
    }
    let palette_max_size = checked_usize(
        context.palette_max_size,
        "palette maximum size is too large",
    )?;
    let mut predictor_reuse_flags = vec![false; context.predictor_palette_size];
    let mut num_predicted_palette_entries = 0usize;
    let mut predictor_entry_idx = 0usize;
    let mut prediction_finished = false;
    while predictor_entry_idx < context.predictor_palette_size
        && !prediction_finished
        && num_predicted_palette_entries < palette_max_size
    {
        let predictor_run = cabac.read_exp_golomb(0)?;
        if predictor_run == 1 {
            prediction_finished = true;
        } else {
            let skip = checked_usize(
                predictor_run.saturating_sub(1),
                "palette predictor run is too large",
            )?;
            predictor_entry_idx =
                predictor_entry_idx
                    .checked_add(skip)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "palette predictor index overflows",
                    ))?;
            if predictor_entry_idx >= context.predictor_palette_size {
                return Err(SyntaxError::InvalidSyntaxValue(
                    "palette predictor run exceeds predictor list",
                ));
            }
            predictor_reuse_flags[predictor_entry_idx] = true;
            num_predicted_palette_entries += 1;
            predictor_entry_idx += 1;
        }
    }
    let num_signalled_palette_entries = if num_predicted_palette_entries < palette_max_size {
        checked_usize(
            cabac.read_exp_golomb(0)?,
            "too many signalled palette entries",
        )?
    } else {
        0
    };
    if num_predicted_palette_entries
        .checked_add(num_signalled_palette_entries)
        .is_none_or(|size| size > palette_max_size)
    {
        return Err(SyntaxError::InvalidSyntaxValue(
            "palette entries exceed palette_max_size",
        ));
    }
    let component_count = if context.chroma_array_type == 0 { 1 } else { 3 };
    let mut new_palette_entries =
        vec![Vec::with_capacity(num_signalled_palette_entries); component_count];
    for (component, entries) in new_palette_entries.iter_mut().enumerate() {
        let bit_depth = if component == 0 {
            bit_depth_luma
        } else {
            bit_depth_chroma
        };
        for _ in 0..num_signalled_palette_entries {
            entries.push(cabac.read_palette_value(bit_depth, context.cu_transquant_bypass_flag)?);
        }
    }
    let current_palette_size = num_predicted_palette_entries
        .checked_add(num_signalled_palette_entries)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "current palette size overflows",
        ))?;
    let palette_escape_val_present_flag = if current_palette_size != 0 {
        cabac.read_bypass_bin()? != 0
    } else {
        false
    };
    let (
        num_palette_indices_minus1,
        palette_index_idc,
        copy_above_indices_for_final_run_flag,
        palette_transpose_flag,
    ) =
        if current_palette_size > 0 {
            let count_minus1 = checked_usize(
                cabac.read_palette_num_indices(current_palette_size as u64)?,
                "palette index count is too large",
            )?;
            let sample_count = context.n_cb_s.checked_mul(context.n_cb_s).ok_or(
                SyntaxError::InvalidSyntaxValue("palette sample count overflows"),
            )?;
            if count_minus1 >= sample_count {
                return Err(SyntaxError::InvalidSyntaxValue(
                    "palette index count exceeds coding block",
                ));
            }
            let mut indices = Vec::with_capacity(count_minus1.saturating_add(1));
            let mut adjust = 0usize;
            for _ in 0..=count_minus1 {
                if current_palette_size > adjust {
                    let index = cabac.read_palette_index(
                        current_palette_size as u64 - u64::from(adjust != 0),
                        adjust == 0,
                    )?;
                    if index > current_palette_size as u64 {
                        return Err(SyntaxError::InvalidSyntaxValue(
                            "palette index exceeds current palette",
                        ));
                    }
                    indices.push(index);
                }
                adjust = 1;
            }
            let copy_final = cabac.read_ae_named(CabacContextTable::PaletteCopyAbove, 0)? != 0;
            let transpose = cabac.read_ae_named(CabacContextTable::PaletteTranspose, 0)? != 0;
            (Some(count_minus1), indices, copy_final, transpose)
        } else {
            (None, Vec::new(), false, false)
        };
    let mut delta_qp = DeltaQpState::new();
    let mut chroma_qp_offset = ChromaQpOffsetState::new();
    if palette_escape_val_present_flag {
        delta_qp.parse(cabac, context.cu_qp_delta_enabled_flag)?;
        if !context.cu_transquant_bypass_flag {
            chroma_qp_offset.parse(
                cabac,
                context.cu_chroma_qp_offset_enabled_flag,
                context.chroma_qp_offset_list_len_minus1,
            )?;
        }
    }
    let sample_count =
        context
            .n_cb_s
            .checked_mul(context.n_cb_s)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "palette sample count overflows",
            ))?;
    let mut runs = Vec::new();
    let mut remaining_indices = num_palette_indices_minus1
        .map(|value| value.saturating_add(1))
        .unwrap_or(0);
    let mut scan_position = 0usize;
    while scan_position < sample_count {
        let copy_above_indices_flag = if current_palette_size > 0
            && scan_position >= context.n_cb_s
            && runs
                .last()
                .is_none_or(|run: &PaletteRunSyntax| !run.copy_above_indices_flag)
        {
            if remaining_indices > 0 && scan_position < sample_count - 1 {
                cabac.read_ae_named(CabacContextTable::PaletteCopyAbove, 0)? != 0
            } else {
                remaining_indices == 0
            }
        } else {
            false
        };
        let palette_index = if current_palette_size > 0 && !copy_above_indices_flag {
            let index = palette_index_idc
                .get(num_palette_indices_minus1.map_or(0, |value| value + 1 - remaining_indices))
                .copied()
                .unwrap_or(0);
            remaining_indices = remaining_indices.saturating_sub(1);
            Some(index)
        } else {
            None
        };
        let max_run_minus1 = sample_count
            .saturating_sub(scan_position)
            .saturating_sub(1)
            .saturating_sub(remaining_indices)
            .saturating_sub(usize::from(copy_above_indices_for_final_run_flag));
        let run_to_end = current_palette_size == 0
            || (remaining_indices == 0
                && copy_above_indices_flag == copy_above_indices_for_final_run_flag);
        let (run_prefix, run_suffix, run_length) = if run_to_end {
            (None, None, max_run_minus1.saturating_add(1))
        } else if max_run_minus1 == 0 {
            (None, None, 1)
        } else {
            let prefix = cabac.read_palette_run_prefix(
                max_run_minus1 as u64,
                palette_index.unwrap_or(0),
                copy_above_indices_flag,
            )?;
            let suffix = if prefix > 1 {
                let boundary = 1_u64.checked_shl((prefix - 1) as u32).unwrap_or(u64::MAX);
                if u64::try_from(max_run_minus1).unwrap_or(u64::MAX) != boundary {
                    Some(cabac.read_palette_run_suffix(max_run_minus1 as u64, prefix)?)
                } else {
                    None
                }
            } else {
                None
            };
            let run_minus1 = if prefix < 2 {
                prefix
            } else {
                1_u64
                    .checked_shl((prefix - 1) as u32)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "palette run prefix is too large",
                    ))?
                    .saturating_add(suffix.unwrap_or(0))
            };
            let length = checked_usize(run_minus1.saturating_add(1), "palette run is too large")?
                .min(max_run_minus1.saturating_add(1));
            (Some(prefix), suffix, length.max(1))
        };
        runs.push(PaletteRunSyntax {
            scan_position,
            copy_above_indices_flag,
            palette_index,
            run_prefix,
            run_suffix,
            run_length,
        });
        scan_position =
            scan_position
                .checked_add(run_length)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "palette scan position overflows",
                ))?;
    }
    let mut escape_values = vec![Vec::new(); component_count];
    if palette_escape_val_present_flag {
        let escape_samples = runs
            .iter()
            .filter(|run| run.palette_index == Some(current_palette_size as u64))
            .map(|run| run.run_length)
            .sum::<usize>();
        for (component, values) in escape_values.iter_mut().enumerate() {
            let bit_depth = if component == 0 {
                bit_depth_luma
            } else {
                bit_depth_chroma
            };
            for _ in 0..escape_samples {
                values
                    .push(cabac.read_palette_value(bit_depth, context.cu_transquant_bypass_flag)?);
            }
        }
    }
    Ok(PaletteCodingSyntax {
        predictor_reuse_flags,
        num_predicted_palette_entries,
        num_signalled_palette_entries,
        new_palette_entries,
        palette_escape_val_present_flag,
        num_palette_indices_minus1,
        palette_index_idc,
        copy_above_indices_for_final_run_flag,
        palette_transpose_flag,
        runs,
        delta_qp,
        chroma_qp_offset,
        escape_values,
    })
}

/// Parses one `prediction_unit()` from §7.3.8.6.
pub fn parse_prediction_unit(
    cabac: &mut impl CabacReader,
    context: PredictionUnitContext,
    cu_skip_flag: bool,
) -> Result<PredictionUnitSyntax, SyntaxError> {
    parse_prediction_unit_with_dimensions(cabac, context, cu_skip_flag, 0, 0, 0)
}

/// Parses one `prediction_unit()` with the dimensions required by the
/// Clause 9.3.3.9 `inter_pred_idc` binarization and its context derivation.
pub fn parse_prediction_unit_with_dimensions(
    cabac: &mut impl CabacReader,
    context: PredictionUnitContext,
    cu_skip_flag: bool,
    n_pb_w: u64,
    n_pb_h: u64,
    ct_depth: usize,
) -> Result<PredictionUnitSyntax, SyntaxError> {
    let max_num_merge_cand = context.five_minus_max_num_merge_cand.checked_add(5).ok_or(
        SyntaxError::InvalidSyntaxValue("merge-candidate count overflows"),
    )?;
    if cu_skip_flag {
        let merge_idx = if max_num_merge_cand > 1 {
            Some(cabac.read_truncated_rice_context(
                CabacContextTable::MergeIdx,
                0,
                1,
                max_num_merge_cand - 1,
                0,
            )?)
        } else {
            None
        };
        return Ok(PredictionUnitSyntax {
            merge_flag: None,
            merge_idx,
            inter_pred_idc: None,
            ref_idx_l0: None,
            mvd_l0: None,
            mvp_l0_flag: None,
            ref_idx_l1: None,
            mvd_l1: None,
            mvp_l1_flag: None,
        });
    }
    let merge_flag = cabac.read_ae_named(CabacContextTable::Merge, 0)? != 0;
    if merge_flag {
        let merge_idx = if max_num_merge_cand > 1 {
            Some(cabac.read_truncated_rice_context(
                CabacContextTable::MergeIdx,
                0,
                1,
                max_num_merge_cand - 1,
                0,
            )?)
        } else {
            None
        };
        return Ok(PredictionUnitSyntax {
            merge_flag: Some(true),
            merge_idx,
            inter_pred_idc: None,
            ref_idx_l0: None,
            mvd_l0: None,
            mvp_l0_flag: None,
            ref_idx_l1: None,
            mvd_l1: None,
            mvp_l1_flag: None,
        });
    }
    let inter_pred_idc = if context.slice_type == 0 {
        Some(cabac.read_inter_pred_idc(n_pb_w, n_pb_h, ct_depth)?)
    } else {
        None
    };
    let uses_l0 = inter_pred_idc != Some(1);
    let uses_l1 = inter_pred_idc == Some(1) || inter_pred_idc == Some(2);
    let ref_idx_l0 = if uses_l0 && context.num_ref_idx_l0_active_minus1 > 0 {
        Some(cabac.read_truncated_rice_context(
            CabacContextTable::RefIdx,
            0,
            2,
            context.num_ref_idx_l0_active_minus1,
            0,
        )?)
    } else {
        None
    };
    let mvd_l0 = if uses_l0 {
        Some(parse_motion_vector_difference(cabac)?)
    } else {
        None
    };
    let mvp_l0_flag = if uses_l0 {
        Some(cabac.read_ae_named(CabacContextTable::MvpFlag, 0)? != 0)
    } else {
        None
    };
    let ref_idx_l1 = if uses_l1 && context.num_ref_idx_l1_active_minus1 > 0 {
        Some(cabac.read_truncated_rice_context(
            CabacContextTable::RefIdx,
            0,
            2,
            context.num_ref_idx_l1_active_minus1,
            0,
        )?)
    } else {
        None
    };
    let mvd_l1 = if uses_l1 && !(context.mvd_l1_zero_flag && inter_pred_idc == Some(2)) {
        Some(parse_motion_vector_difference(cabac)?)
    } else {
        None
    };
    let mvp_l1_flag = if uses_l1 {
        Some(cabac.read_ae_named(CabacContextTable::MvpFlag, 0)? != 0)
    } else {
        None
    };
    Ok(PredictionUnitSyntax {
        merge_flag: Some(false),
        merge_idx: None,
        inter_pred_idc,
        ref_idx_l0,
        mvd_l0,
        mvp_l0_flag,
        ref_idx_l1,
        mvd_l1,
        mvp_l1_flag,
    })
}

fn block_size(log2_cb_size: u8) -> Result<u64, SyntaxError> {
    1u64.checked_shl(u32::from(log2_cb_size))
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "coding-block size shift is too large",
        ))
}

/// Parses the recursive split structure of `coding_quadtree()`.
///
/// Coding-unit leaf syntax is supplied by the caller in later Clause 7
/// parsers; this function consumes and represents the complete quadtree split
/// decisions and preserves the standard child ordering.
pub fn parse_coding_quadtree_shape(
    cabac: &mut impl CabacReader,
    x: u64,
    y: u64,
    log2_cb_size: u8,
    cqt_depth: u32,
    geometry: CodingQuadtreeGeometry,
) -> Result<CodingQuadtreeNode, SyntaxError> {
    let size = block_size(log2_cb_size)?;
    let right = x.checked_add(size).ok_or(SyntaxError::InvalidSyntaxValue(
        "coding-block x coordinate overflows",
    ))?;
    let bottom = y.checked_add(size).ok_or(SyntaxError::InvalidSyntaxValue(
        "coding-block y coordinate overflows",
    ))?;
    let split_allowed = right <= geometry.pic_width_in_luma_samples
        && bottom <= geometry.pic_height_in_luma_samples
        && log2_cb_size > geometry.min_cb_log2_size;
    let split_cu_flag = if split_allowed {
        cabac.read_ae_named(CabacContextTable::SplitCu, 0)? != 0
    } else {
        false
    };
    let mut children = Vec::new();
    if split_cu_flag {
        let child_size = size / 2;
        let child_log2 = log2_cb_size
            .checked_sub(1)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "coding-quadtree depth underflows",
            ))?;
        let child_positions = [
            (x, y),
            (x + child_size, y),
            (x, y + child_size),
            (x + child_size, y + child_size),
        ];
        for (child_x, child_y) in child_positions {
            if child_x < geometry.pic_width_in_luma_samples
                && child_y < geometry.pic_height_in_luma_samples
            {
                children.push(parse_coding_quadtree_shape(
                    cabac,
                    child_x,
                    child_y,
                    child_log2,
                    cqt_depth + 1,
                    geometry,
                )?);
            }
        }
    }
    Ok(CodingQuadtreeNode {
        x,
        y,
        log2_cb_size,
        cqt_depth,
        split_cu_flag,
        children,
    })
}

/// A complete coding-quadtree node, including the coding-unit syntax at a
/// leaf. `CodingQuadtreeNode` remains available for callers that only need
/// split geometry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodingTreeNodeSyntax {
    /// Luma-space node origin.
    pub x: u64,
    /// Luma-space node origin.
    pub y: u64,
    /// Coding-block log2 size.
    pub log2_cb_size: u8,
    /// Coding-quadtree depth.
    pub cqt_depth: u32,
    /// Whether the node was split.
    pub split_cu_flag: bool,
    /// Child nodes in the order specified by §7.3.8.4.
    pub children: Vec<CodingTreeNodeSyntax>,
    /// Coding-unit syntax at an unsplit leaf.
    pub coding_unit: Option<CodingUnitSyntax>,
}

#[derive(Clone, Copy, Debug)]
struct CodedBlockNeighbour {
    x: u64,
    y: u64,
    size: u64,
    cqt_depth: u32,
    cu_skip_flag: bool,
    ctb_addr_in_rs: usize,
}

#[derive(Default)]
struct CodingNeighbourState {
    blocks: Vec<CodedBlockNeighbour>,
    current_ctb: Option<(u64, u64, u64, usize, bool, bool)>,
}

impl CodingNeighbourState {
    fn begin_ctu(
        &mut self,
        x: u64,
        y: u64,
        size: u64,
        ctb_addr_in_rs: usize,
        left_available: bool,
        up_available: bool,
    ) {
        self.current_ctb = Some((x, y, size, ctb_addr_in_rs, left_available, up_available));
    }

    fn find(&self, x: u64, y: u64, left: bool) -> Option<CodedBlockNeighbour> {
        let (ctb_x, ctb_y, ctb_size, current_ctb, left_available, up_available) =
            self.current_ctb?;
        let target_x = if left { x.checked_sub(1)? } else { x };
        let target_y = if left { y } else { y.checked_sub(1)? };
        if left && target_x < ctb_x && !left_available {
            return None;
        }
        if !left && target_y < ctb_y && !up_available {
            return None;
        }
        if target_x >= ctb_x + ctb_size || target_y >= ctb_y + ctb_size {
            return None;
        }
        self.blocks
            .iter()
            .rev()
            .find(|block| {
                block.x <= target_x
                    && block.y <= target_y
                    && target_x < block.x + block.size
                    && target_y < block.y + block.size
                    && (block.ctb_addr_in_rs == current_ctb
                        || (left && target_x < ctb_x)
                        || (!left && target_y < ctb_y))
            })
            .copied()
    }

    fn record(
        &mut self,
        x: u64,
        y: u64,
        size: u64,
        cqt_depth: u32,
        cu_skip_flag: bool,
        ctb_addr_in_rs: usize,
    ) {
        self.blocks.push(CodedBlockNeighbour {
            x,
            y,
            size,
            cqt_depth,
            cu_skip_flag,
            ctb_addr_in_rs,
        });
    }
}

fn split_context_increment(state: &CodingNeighbourState, x: u64, y: u64, depth: u32) -> usize {
    usize::from(
        state
            .find(x, y, true)
            .is_some_and(|block| block.cqt_depth > depth),
    ) + usize::from(
        state
            .find(x, y, false)
            .is_some_and(|block| block.cqt_depth > depth),
    )
}

fn skip_context_increment(state: &CodingNeighbourState, x: u64, y: u64) -> usize {
    usize::from(
        state
            .find(x, y, true)
            .is_some_and(|block| block.cu_skip_flag),
    ) + usize::from(
        state
            .find(x, y, false)
            .is_some_and(|block| block.cu_skip_flag),
    )
}

/// Parses `coding_quadtree()` and `coding_unit()` recursively.
pub fn parse_coding_quadtree(
    cabac: &mut impl CabacReader,
    x: u64,
    y: u64,
    log2_cb_size: u8,
    cqt_depth: u32,
    geometry: CodingQuadtreeGeometry,
    coding_unit_context: CodingUnitContext,
) -> Result<CodingTreeNodeSyntax, SyntaxError> {
    let size = block_size(log2_cb_size)?;
    let mut state = CodingNeighbourState::default();
    state.begin_ctu(x, y, size, usize::MAX, false, false);
    parse_coding_quadtree_inner(
        cabac,
        x,
        y,
        log2_cb_size,
        cqt_depth,
        geometry,
        coding_unit_context,
        &mut state,
        usize::MAX,
        8,
        8,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn parse_coding_quadtree_inner(
    cabac: &mut impl CabacReader,
    x: u64,
    y: u64,
    log2_cb_size: u8,
    cqt_depth: u32,
    geometry: CodingQuadtreeGeometry,
    mut coding_unit_context: CodingUnitContext,
    state: &mut CodingNeighbourState,
    ctb_addr_in_rs: usize,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
    amp_enabled: bool,
) -> Result<CodingTreeNodeSyntax, SyntaxError> {
    let size = block_size(log2_cb_size)?;
    let right = x.checked_add(size).ok_or(SyntaxError::InvalidSyntaxValue(
        "coding-block x coordinate overflows",
    ))?;
    let bottom = y.checked_add(size).ok_or(SyntaxError::InvalidSyntaxValue(
        "coding-block y coordinate overflows",
    ))?;
    let split_allowed = right <= geometry.pic_width_in_luma_samples
        && bottom <= geometry.pic_height_in_luma_samples
        && log2_cb_size > geometry.min_cb_log2_size;
    let split_cu_flag = if split_allowed {
        cabac.read_ae_named(
            CabacContextTable::SplitCu,
            split_context_increment(state, x, y, cqt_depth),
        )? != 0
    } else {
        false
    };
    let mut children = Vec::new();
    let coding_unit = if split_cu_flag {
        let child_size = size / 2;
        let child_log2 = log2_cb_size
            .checked_sub(1)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "coding-quadtree depth underflows",
            ))?;
        let child_positions = [
            (x, y),
            (x + child_size, y),
            (x, y + child_size),
            (x + child_size, y + child_size),
        ];
        for (child_x, child_y) in child_positions {
            if child_x < geometry.pic_width_in_luma_samples
                && child_y < geometry.pic_height_in_luma_samples
            {
                children.push(parse_coding_quadtree_inner(
                    cabac,
                    child_x,
                    child_y,
                    child_log2,
                    cqt_depth + 1,
                    geometry,
                    coding_unit_context,
                    state,
                    ctb_addr_in_rs,
                    bit_depth_luma,
                    bit_depth_chroma,
                    amp_enabled,
                )?);
            }
        }
        None
    } else {
        coding_unit_context.log2_cb_size = log2_cb_size;
        let skip_ctx_inc = if coding_unit_context.slice_type != 2 {
            skip_context_increment(state, x, y)
        } else {
            0
        };
        Some(parse_coding_unit_with_skip_context_and_bit_depth(
            cabac,
            coding_unit_context,
            skip_ctx_inc,
            bit_depth_luma,
            bit_depth_chroma,
            cqt_depth,
            amp_enabled,
        )?)
    };
    if let Some(ref coding_unit) = coding_unit {
        state.record(
            x,
            y,
            size,
            cqt_depth,
            coding_unit.cu_skip_flag == Some(true),
            ctb_addr_in_rs,
        );
    }
    Ok(CodingTreeNodeSyntax {
        x,
        y,
        log2_cb_size,
        cqt_depth,
        split_cu_flag,
        children,
        coding_unit,
    })
}

/// One parsed coding-tree unit from general slice-segment data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodingTreeUnitSyntax {
    /// `CtbAddrInTs` for this unit.
    pub ctb_addr_in_ts: usize,
    /// `CtbAddrInRs` for this unit.
    pub ctb_addr_in_rs: usize,
    /// SAO syntax, when either slice SAO flag is enabled.
    pub sao: Option<SaoSyntax>,
    /// Recursive coding-quadtree syntax.
    pub coding_tree: CodingTreeNodeSyntax,
    /// `end_of_slice_segment_flag` following this CTU.
    pub end_of_slice_segment_flag: bool,
}

/// Context for `slice_segment_data()` addressing and CTU syntax.
#[derive(Clone, Debug)]
pub struct SliceSegmentDataContext<'a> {
    /// First `CtbAddrInTs` of the slice segment.
    pub start_ctb_addr_in_ts: usize,
    /// `PicWidthInCtbsY`.
    pub pic_width_in_ctbs: usize,
    /// `SliceAddrRs`.
    pub slice_addr_rs: usize,
    /// `tiles_enabled_flag`.
    pub tiles_enabled_flag: bool,
    /// `entropy_coding_sync_enabled_flag`.
    pub entropy_coding_sync_enabled_flag: bool,
    /// Tile identifier indexed by tile-scan address.
    pub tile_ids: &'a [u64],
    /// `CtbAddrTsToRs` mapping.
    pub ctb_addr_in_ts_to_rs: &'a [usize],
    /// `CtbAddrRsToTs` mapping.
    pub ctb_addr_rs_to_ts: &'a [usize],
    /// Whether luma SAO is enabled for the slice.
    pub slice_sao_luma_flag: bool,
    /// Whether chroma SAO is enabled for the slice.
    pub slice_sao_chroma_flag: bool,
    /// Whether the chroma array type is non-zero.
    pub chroma_array_type_nonzero: bool,
    /// CTB geometry.
    pub geometry: CodingQuadtreeGeometry,
    /// Coding-unit condition flags.
    pub coding_unit: CodingUnitContext,
}

/// Parsed `slice_segment_data()` including all CTUs and subset boundaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceSegmentDataSyntax {
    /// Parsed CTUs in tile-scan order.
    pub coding_tree_units: Vec<CodingTreeUnitSyntax>,
    /// Number of `end_of_subset_one_bit` values consumed.
    pub subset_boundary_count: usize,
}

/// Parsed `slice_segment_layer_rbsp()` from §7.3.2.9.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceSegmentLayerSyntax {
    /// General or dependent slice-segment header.
    pub header: SliceSegmentHeaderSyntax,
    /// Slice-segment data for an independent segment.
    pub data: Option<SliceSegmentDataSyntax>,
    /// Number of trailing `cabac_zero_word` values.
    pub cabac_zero_word_count: usize,
}

/// Parses the complete VCL RBSP composition boundary from §7.3.2.9.
pub fn parse_slice_segment_layer_rbsp(
    reader: &mut BitReader<'_>,
    cabac: &mut impl CabacReader,
    header_context: &SliceSegmentHeaderContext<'_>,
    data_context: Option<SliceSegmentDataContext<'_>>,
) -> Result<SliceSegmentLayerSyntax, SyntaxError> {
    parse_slice_segment_layer_rbsp_with_bit_depth(
        reader,
        cabac,
        header_context,
        data_context,
        10,
        10,
    )
}

/// Parses a slice-segment layer with explicit luma and chroma bit depths.
pub fn parse_slice_segment_layer_rbsp_with_bit_depth(
    reader: &mut BitReader<'_>,
    cabac: &mut impl CabacReader,
    header_context: &SliceSegmentHeaderContext<'_>,
    data_context: Option<SliceSegmentDataContext<'_>>,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
) -> Result<SliceSegmentLayerSyntax, SyntaxError> {
    parse_slice_segment_layer_rbsp_with_bit_depth_and_amp(
        reader,
        cabac,
        header_context,
        data_context,
        bit_depth_luma,
        bit_depth_chroma,
        false,
    )
}

/// Parses a slice-segment layer with explicit bit depths and the SPS
/// `amp_enabled_flag` supplied to coding-unit `part_mode` parsing.
pub fn parse_slice_segment_layer_rbsp_with_bit_depth_and_amp(
    reader: &mut BitReader<'_>,
    cabac: &mut impl CabacReader,
    header_context: &SliceSegmentHeaderContext<'_>,
    data_context: Option<SliceSegmentDataContext<'_>>,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
    amp_enabled: bool,
) -> Result<SliceSegmentLayerSyntax, SyntaxError> {
    let header = crate::syntax::parse_slice_segment_header(reader, header_context)?;
    let data = if header.dependent_slice_segment_flag {
        None
    } else {
        let context = data_context.ok_or(SyntaxError::InvalidSyntaxValue(
            "independent slice segment requires slice-data context",
        ))?;
        Some(parse_slice_segment_data_with_bit_depth_and_amp(
            cabac,
            context,
            bit_depth_luma,
            bit_depth_chroma,
            amp_enabled,
        )?)
    };
    let cabac_zero_word_count = cabac.rbsp_slice_segment_trailing_bits()?;
    Ok(SliceSegmentLayerSyntax {
        header,
        data,
        cabac_zero_word_count,
    })
}

fn map_at(values: &[usize], index: usize, name: &'static str) -> Result<usize, SyntaxError> {
    values
        .get(index)
        .copied()
        .ok_or(SyntaxError::InvalidSyntaxValue(name))
}

fn tile_at(values: &[u64], index: usize) -> Result<u64, SyntaxError> {
    values
        .get(index)
        .copied()
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "tile mapping is incomplete",
        ))
}

/// Parses the complete general slice-segment data loop from §7.3.8.1.
pub fn parse_slice_segment_data(
    cabac: &mut impl CabacReader,
    context: SliceSegmentDataContext<'_>,
) -> Result<SliceSegmentDataSyntax, SyntaxError> {
    parse_slice_segment_data_with_bit_depth(cabac, context, 10, 10)
}

/// Parses slice-segment data with explicit luma and chroma bit depths.
pub fn parse_slice_segment_data_with_bit_depth(
    cabac: &mut impl CabacReader,
    context: SliceSegmentDataContext<'_>,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
) -> Result<SliceSegmentDataSyntax, SyntaxError> {
    parse_slice_segment_data_with_bit_depth_and_amp(
        cabac,
        context,
        bit_depth_luma,
        bit_depth_chroma,
        false,
    )
}

/// Parses slice-segment data with explicit bit depths and the SPS
/// `amp_enabled_flag` used by all coding units in the segment.
pub fn parse_slice_segment_data_with_bit_depth_and_amp(
    cabac: &mut impl CabacReader,
    context: SliceSegmentDataContext<'_>,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
    amp_enabled: bool,
) -> Result<SliceSegmentDataSyntax, SyntaxError> {
    let mut ctb_addr_in_ts = context.start_ctb_addr_in_ts;
    let mut coding_tree_units = Vec::new();
    let mut subset_boundary_count = 0;
    let mut neighbour_state = CodingNeighbourState::default();
    loop {
        let ctb_addr_in_rs = map_at(
            context.ctb_addr_in_ts_to_rs,
            ctb_addr_in_ts,
            "tile-scan mapping is incomplete",
        )?;
        let rx = ctb_addr_in_rs % context.pic_width_in_ctbs;
        let ry = ctb_addr_in_rs / context.pic_width_in_ctbs;
        let current_tile = tile_at(context.tile_ids, ctb_addr_in_ts)?;
        let left_available = if rx > 0 && ctb_addr_in_rs > context.slice_addr_rs {
            let left_ts = map_at(
                context.ctb_addr_rs_to_ts,
                ctb_addr_in_rs - 1,
                "raster-to-tile mapping is incomplete",
            )?;
            tile_at(context.tile_ids, left_ts)? == current_tile
        } else {
            false
        };
        let up_available = if ry > 0 && ctb_addr_in_rs >= context.slice_addr_rs {
            let up_rs = ctb_addr_in_rs - context.pic_width_in_ctbs;
            let up_ts = map_at(
                context.ctb_addr_rs_to_ts,
                up_rs,
                "raster-to-tile mapping is incomplete",
            )?;
            tile_at(context.tile_ids, up_ts)? == current_tile && up_rs >= context.slice_addr_rs
        } else {
            false
        };
        let sao = if context.slice_sao_luma_flag || context.slice_sao_chroma_flag {
            Some(parse_sao_with_bit_depth(
                cabac,
                left_available,
                up_available,
                context.slice_sao_luma_flag,
                context.slice_sao_chroma_flag,
                context.chroma_array_type_nonzero,
                bit_depth_luma,
                bit_depth_chroma,
            )?)
        } else {
            None
        };
        let ctb_x = (rx as u64) << context.coding_unit.log2_cb_size;
        let ctb_y = (ry as u64) << context.coding_unit.log2_cb_size;
        let ctb_size = block_size(context.coding_unit.log2_cb_size)?;
        neighbour_state.begin_ctu(
            ctb_x,
            ctb_y,
            ctb_size,
            ctb_addr_in_rs,
            left_available,
            up_available,
        );
        let coding_tree = parse_coding_quadtree_inner(
            cabac,
            ctb_x,
            ctb_y,
            context.coding_unit.log2_cb_size,
            0,
            context.geometry,
            context.coding_unit,
            &mut neighbour_state,
            ctb_addr_in_rs,
            bit_depth_luma,
            bit_depth_chroma,
            amp_enabled,
        )?;
        let store_wpp_state = context.entropy_coding_sync_enabled_flag
            && (ctb_addr_in_rs % context.pic_width_in_ctbs == 1
                || (ctb_addr_in_rs > 1
                    && tile_at(context.tile_ids, ctb_addr_in_ts)?
                        != tile_at(
                            context.tile_ids,
                            map_at(
                                context.ctb_addr_rs_to_ts,
                                ctb_addr_in_rs - 2,
                                "raster-to-tile mapping is incomplete",
                            )?,
                        )?));
        if store_wpp_state {
            cabac.store_wpp_contexts();
        }
        let end_of_slice_segment_flag = cabac.read_terminate()? != 0;
        coding_tree_units.push(CodingTreeUnitSyntax {
            ctb_addr_in_ts,
            ctb_addr_in_rs,
            sao,
            coding_tree,
            end_of_slice_segment_flag,
        });
        if end_of_slice_segment_flag {
            break;
        }
        ctb_addr_in_ts = ctb_addr_in_ts
            .checked_add(1)
            .ok_or(SyntaxError::InvalidSyntaxValue("CTU address overflows"))?;
        let next_rs = map_at(
            context.ctb_addr_in_ts_to_rs,
            ctb_addr_in_ts,
            "slice segment has no following CTU",
        )?;
        let crosses_tile = context.tiles_enabled_flag
            && tile_at(context.tile_ids, ctb_addr_in_ts)? != current_tile;
        let crosses_sync = context.entropy_coding_sync_enabled_flag
            && (next_rs % context.pic_width_in_ctbs == 0
                || tile_at(
                    context.tile_ids,
                    map_at(
                        context.ctb_addr_rs_to_ts,
                        next_rs.saturating_sub(1),
                        "raster-to-tile mapping is incomplete",
                    )?,
                )? != tile_at(context.tile_ids, ctb_addr_in_ts)?);
        if crosses_tile || crosses_sync {
            if cabac.read_terminate()? != 1 {
                return Err(SyntaxError::InvalidSyntaxValue(
                    "end_of_subset_one_bit must equal one",
                ));
            }
            cabac.byte_alignment()?;
            if crosses_tile {
                cabac.reset_contexts_to_initial();
            } else if crosses_sync {
                let next_ry = next_rs / context.pic_width_in_ctbs;
                let top_available = if next_ry == 0 || next_rs < context.slice_addr_rs {
                    false
                } else {
                    let next_up_rs = next_rs - context.pic_width_in_ctbs;
                    let next_up_ts = map_at(
                        context.ctb_addr_rs_to_ts,
                        next_up_rs,
                        "raster-to-tile mapping is incomplete",
                    )?;
                    next_up_rs >= context.slice_addr_rs
                        && tile_at(context.tile_ids, next_up_ts)?
                            == tile_at(context.tile_ids, ctb_addr_in_ts)?
                };
                if top_available {
                    if !cabac.synchronize_wpp_contexts() {
                        cabac.reset_contexts_to_initial();
                    }
                } else {
                    cabac.reset_contexts_to_initial();
                }
            }
            cabac.initialize_arithmetic_engine()?;
            subset_boundary_count += 1;
        }
    }
    Ok(SliceSegmentDataSyntax {
        coding_tree_units,
        subset_boundary_count,
    })
}

/// Parsed motion-vector-difference syntax from §7.3.8.9.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotionVectorDifferenceSyntax {
    /// `abs_mvd_greater0_flag` values.
    pub abs_mvd_greater0_flag: [bool; 2],
    /// `abs_mvd_greater1_flag` values, inferred false when absent.
    pub abs_mvd_greater1_flag: [bool; 2],
    /// `abs_mvd_minus2` values, when greater-than-one is set.
    pub abs_mvd_minus2: [Option<u64>; 2],
    /// `mvd_sign_flag` values, when the absolute value is non-zero.
    pub mvd_sign_flag: [Option<bool>; 2],
}

/// Parses `mvd_coding()`.
pub fn parse_motion_vector_difference(
    cabac: &mut impl CabacReader,
) -> Result<MotionVectorDifferenceSyntax, SyntaxError> {
    let abs_mvd_greater0_flag = [
        cabac.read_ae_named(CabacContextTable::AbsMvd, 0)? != 0,
        cabac.read_ae_named(CabacContextTable::AbsMvd, 0)? != 0,
    ];
    let mut abs_mvd_greater1_flag = [false; 2];
    for component in 0..2 {
        if abs_mvd_greater0_flag[component] {
            abs_mvd_greater1_flag[component] =
                cabac.read_ae_named(CabacContextTable::AbsMvd, 1)? != 0;
        }
    }
    let mut abs_mvd_minus2 = [None; 2];
    let mut mvd_sign_flag = [None; 2];
    for component in 0..2 {
        if abs_mvd_greater0_flag[component] {
            if abs_mvd_greater1_flag[component] {
                abs_mvd_minus2[component] = Some(cabac.read_exp_golomb(1)?);
            }
            mvd_sign_flag[component] = Some(cabac.read_bypass_bin()? != 0);
        }
    }
    Ok(MotionVectorDifferenceSyntax {
        abs_mvd_greater0_flag,
        abs_mvd_greater1_flag,
        abs_mvd_minus2,
        mvd_sign_flag,
    })
}

/// Cross-component prediction syntax from §7.3.8.12.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrossComponentPredictionSyntax {
    /// `log2_res_scale_abs_plus1`.
    pub log2_res_scale_abs_plus1: u64,
    /// `res_scale_sign_flag`, when the absolute scale is non-zero.
    pub res_scale_sign_flag: Option<bool>,
}

/// Parses `cross_comp_pred()`.
pub fn parse_cross_component_prediction(
    cabac: &mut impl CabacReader,
) -> Result<CrossComponentPredictionSyntax, SyntaxError> {
    let log2_res_scale_abs_plus1 =
        cabac.read_truncated_rice_context(CabacContextTable::Log2ResScaleAbsPlus1, 0, 4, 4, 0)?;
    let res_scale_sign_flag = if log2_res_scale_abs_plus1 != 0 {
        Some(cabac.read_ae_named(CabacContextTable::ResScaleSign, 0)? != 0)
    } else {
        None
    };
    Ok(CrossComponentPredictionSyntax {
        log2_res_scale_abs_plus1,
        res_scale_sign_flag,
    })
}

/// Stateful CU delta-QP syntax from §7.3.8.14.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaQpState {
    /// Whether the syntax has already been coded for the current CU.
    pub coded: bool,
    /// Signed `CuQpDeltaVal`.
    pub value: i64,
}

impl DeltaQpState {
    /// Creates the uncoded zero state specified at the start of a CU.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            coded: false,
            value: 0,
        }
    }

    /// Parses `delta_qp()` when CU QP deltas are enabled.
    pub fn parse(
        &mut self,
        cabac: &mut impl CabacReader,
        enabled: bool,
    ) -> Result<(), SyntaxError> {
        if enabled && !self.coded {
            self.coded = true;
            let absolute = cabac.read_cu_qp_delta_abs()?;
            if absolute == 0 {
                self.value = 0;
            } else {
                let negative = cabac.read_bypass_bin()? != 0;
                let absolute = i64::try_from(absolute)
                    .map_err(|_| SyntaxError::InvalidSyntaxValue("CU delta QP is too large"))?;
                self.value = if negative { -absolute } else { absolute };
            }
        }
        Ok(())
    }
}

impl Default for DeltaQpState {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateful chroma QP-offset syntax from §7.3.8.15.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromaQpOffsetState {
    /// Whether the syntax has already been coded for the current CU.
    pub coded: bool,
    /// `cu_chroma_qp_offset_flag`.
    pub enabled: bool,
    /// `cu_chroma_qp_offset_idx`, when present.
    pub index: Option<u64>,
}

impl ChromaQpOffsetState {
    /// Creates the uncoded state specified at the start of a CU.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            coded: false,
            enabled: false,
            index: None,
        }
    }

    /// Parses `chroma_qp_offset()` when the PPS enables the list.
    pub fn parse(
        &mut self,
        cabac: &mut impl CabacReader,
        cu_chroma_qp_offset_enabled_flag: bool,
        chroma_qp_offset_list_len_minus1: u64,
    ) -> Result<(), SyntaxError> {
        if cu_chroma_qp_offset_enabled_flag && !self.coded {
            self.coded = true;
            self.enabled = cabac.read_ae_named(CabacContextTable::ChromaQpOffsetFlag, 0)? != 0;
            if self.enabled && chroma_qp_offset_list_len_minus1 > 0 {
                self.index = Some(cabac.read_truncated_rice_repeated_context(
                    CabacContextTable::ChromaQpOffsetIdx,
                    0,
                    5,
                    chroma_qp_offset_list_len_minus1,
                    0,
                )?);
            }
        }
        Ok(())
    }
}

impl Default for ChromaQpOffsetState {
    fn default() -> Self {
        Self::new()
    }
}

/// PCM sample syntax from §7.3.8.7.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcmSampleSyntax {
    /// Luma PCM samples in raster order.
    pub luma_samples: Vec<u64>,
    /// Chroma PCM samples in component/raster order.
    pub chroma_samples: Vec<u64>,
}

/// Parses the fixed-length `pcm_sample()` syntax after CABAC alignment.
pub fn parse_pcm_sample(
    reader: &mut BitReader<'_>,
    luma_sample_count: usize,
    chroma_sample_count: usize,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
) -> Result<PcmSampleSyntax, SyntaxError> {
    while !reader.byte_aligned() {
        if reader.read_u(1)? != 0 {
            return Err(SyntaxError::InvalidAlignmentZero);
        }
    }
    let mut luma_samples = Vec::with_capacity(luma_sample_count);
    for _ in 0..luma_sample_count {
        luma_samples.push(reader.read_u(bit_depth_luma)?);
    }
    let mut chroma_samples = Vec::with_capacity(chroma_sample_count);
    for _ in 0..chroma_sample_count {
        chroma_samples.push(reader.read_u(bit_depth_chroma)?);
    }
    Ok(PcmSampleSyntax {
        luma_samples,
        chroma_samples,
    })
}

/// Parses PCM samples through the active CABAC reader, including the
/// post-PCM arithmetic-decoder initialization required by Clause 9.
pub fn parse_pcm_sample_from_cabac(
    cabac: &mut impl CabacReader,
    luma_sample_count: usize,
    chroma_sample_count: usize,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
) -> Result<PcmSampleSyntax, SyntaxError> {
    cabac.read_pcm_samples(
        luma_sample_count,
        chroma_sample_count,
        bit_depth_luma,
        bit_depth_chroma,
    )
}

/// One parsed SAO component from §7.3.8.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaoComponentSyntax {
    /// `sao_type_idx`.
    pub type_idx: u64,
    /// Four `sao_offset_abs` values.
    pub offset_abs: [u64; 4],
    /// Sign flags for non-zero offsets in edge-offset mode.
    pub offset_sign: Vec<bool>,
    /// `sao_band_position`, for band-offset mode.
    pub band_position: Option<u64>,
    /// `sao_eo_class`, for edge-offset mode.
    pub eo_class: Option<u64>,
}

/// Parsed SAO syntax from §7.3.8.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaoSyntax {
    /// `sao_merge_left_flag`, when available.
    pub merge_left_flag: bool,
    /// `sao_merge_up_flag`, when available and not merged left.
    pub merge_up_flag: bool,
    /// Luma component syntax, when enabled.
    pub luma: Option<SaoComponentSyntax>,
    /// Chroma component syntax, when enabled.
    pub chroma: Option<SaoComponentSyntax>,
}

fn parse_sao_component(
    cabac: &mut impl CabacReader,
    bit_depth: usize,
) -> Result<SaoComponentSyntax, SyntaxError> {
    let type_idx = cabac.read_truncated_rice_context(CabacContextTable::SaoType, 0, 1, 2, 0)?;
    let mut offset_abs = [0; 4];
    let offset_c_max = (1_u64
        << u32::try_from(bit_depth.min(10).saturating_sub(5))
            .map_err(|_| SyntaxError::InvalidSyntaxValue("SAO bit depth is out of range"))?)
        - 1;
    for value in &mut offset_abs {
        *value = cabac.read_truncated_rice(offset_c_max, 0)?;
    }
    let mut offset_sign = Vec::new();
    let (band_position, eo_class) = if type_idx == 1 {
        for &value in &offset_abs {
            if value != 0 {
                offset_sign.push(cabac.read_bypass_bin()? != 0);
            }
        }
        (Some(cabac.read_fixed_bypass(31)?), None)
    } else if type_idx != 0 {
        (None, Some(cabac.read_fixed_bypass(3)?))
    } else {
        (None, None)
    };
    Ok(SaoComponentSyntax {
        type_idx,
        offset_abs,
        offset_sign,
        band_position,
        eo_class,
    })
}

/// Parses `sao(rx, ry)` with caller-provided neighbour availability.
pub fn parse_sao(
    cabac: &mut impl CabacReader,
    left_available: bool,
    up_available: bool,
    slice_sao_luma_flag: bool,
    slice_sao_chroma_flag: bool,
    chroma_array_type_nonzero: bool,
) -> Result<SaoSyntax, SyntaxError> {
    parse_sao_with_bit_depth(
        cabac,
        left_available,
        up_available,
        slice_sao_luma_flag,
        slice_sao_chroma_flag,
        chroma_array_type_nonzero,
        10,
        10,
    )
}

/// Parses `sao(rx, ry)` with explicit luma and chroma bit depths.
#[allow(clippy::too_many_arguments)]
pub fn parse_sao_with_bit_depth(
    cabac: &mut impl CabacReader,
    left_available: bool,
    up_available: bool,
    slice_sao_luma_flag: bool,
    slice_sao_chroma_flag: bool,
    chroma_array_type_nonzero: bool,
    bit_depth_luma: usize,
    bit_depth_chroma: usize,
) -> Result<SaoSyntax, SyntaxError> {
    let merge_left_flag = if left_available {
        cabac.read_ae_named(CabacContextTable::SaoMerge, 0)? != 0
    } else {
        false
    };
    let merge_up_flag = if up_available && !merge_left_flag {
        cabac.read_ae_named(CabacContextTable::SaoMerge, 0)? != 0
    } else {
        false
    };
    if merge_left_flag || merge_up_flag {
        return Ok(SaoSyntax {
            merge_left_flag,
            merge_up_flag,
            luma: None,
            chroma: None,
        });
    }
    let luma = if slice_sao_luma_flag {
        Some(parse_sao_component(cabac, bit_depth_luma)?)
    } else {
        None
    };
    let chroma = if slice_sao_chroma_flag && chroma_array_type_nonzero {
        Some(parse_sao_component(cabac, bit_depth_chroma)?)
    } else {
        None
    };
    Ok(SaoSyntax {
        merge_left_flag,
        merge_up_flag,
        luma,
        chroma,
    })
}
