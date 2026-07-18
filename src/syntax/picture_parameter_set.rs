use super::{BitReader, ScalingListData, SyntaxError};

/// Tile syntax embedded in a picture parameter set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsTileSyntax {
    /// `num_tile_columns_minus1`.
    pub num_tile_columns_minus1: u64,
    /// `num_tile_rows_minus1`.
    pub num_tile_rows_minus1: u64,
    /// `uniform_spacing_flag`.
    pub uniform_spacing_flag: bool,
    /// `column_width_minus1` values for non-uniform spacing.
    pub column_width_minus1: Vec<u64>,
    /// `row_height_minus1` values for non-uniform spacing.
    pub row_height_minus1: Vec<u64>,
    /// `loop_filter_across_tiles_enabled_flag`.
    pub loop_filter_across_tiles_enabled_flag: bool,
}

/// Deblocking-control syntax in a picture parameter set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PpsDeblockingFilterSyntax {
    /// `deblocking_filter_override_enabled_flag`.
    pub deblocking_filter_override_enabled_flag: bool,
    /// `pps_deblocking_filter_disabled_flag`.
    pub pps_deblocking_filter_disabled_flag: bool,
    /// `pps_beta_offset_div2`, when deblocking is enabled.
    pub pps_beta_offset_div2: Option<i64>,
    /// `pps_tc_offset_div2`, when deblocking is enabled.
    pub pps_tc_offset_div2: Option<i64>,
}

/// PPS range-extension syntax from §7.3.2.3.2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsRangeExtensionSyntax {
    /// `log2_max_transform_skip_block_size_minus2`, when transform skip is enabled.
    pub log2_max_transform_skip_block_size_minus2: Option<u64>,
    /// `cross_component_prediction_enabled_flag`.
    pub cross_component_prediction_enabled_flag: bool,
    /// `chroma_qp_offset_list_enabled_flag`.
    pub chroma_qp_offset_list_enabled_flag: bool,
    /// `diff_cu_chroma_qp_offset_depth`, when the list is enabled.
    pub diff_cu_chroma_qp_offset_depth: Option<u64>,
    /// `chroma_qp_offset_list_len_minus1`, when the list is enabled.
    pub chroma_qp_offset_list_len_minus1: Option<u64>,
    /// `cb_qp_offset_list` values.
    pub cb_qp_offset_list: Vec<i64>,
    /// `cr_qp_offset_list` values.
    pub cr_qp_offset_list: Vec<i64>,
    /// `log2_sao_offset_scale_luma`.
    pub log2_sao_offset_scale_luma: u64,
    /// `log2_sao_offset_scale_chroma`.
    pub log2_sao_offset_scale_chroma: u64,
}

/// PPS screen-content-coding extension syntax from §7.3.2.3.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsSccExtensionSyntax {
    /// `pps_curr_pic_ref_enabled_flag`.
    pub pps_curr_pic_ref_enabled_flag: bool,
    /// `residual_adaptive_colour_transform_enabled_flag`.
    pub residual_adaptive_colour_transform_enabled_flag: bool,
    /// `pps_slice_act_qp_offsets_present_flag`, when residual ACT is enabled.
    pub pps_slice_act_qp_offsets_present_flag: Option<bool>,
    /// `pps_act_y_qp_offset_plus5`, when residual ACT is enabled.
    pub pps_act_y_qp_offset_plus5: Option<i64>,
    /// `pps_act_cb_qp_offset_plus5`, when residual ACT is enabled.
    pub pps_act_cb_qp_offset_plus5: Option<i64>,
    /// `pps_act_cr_qp_offset_plus3`, when residual ACT is enabled.
    pub pps_act_cr_qp_offset_plus3: Option<i64>,
    /// `pps_palette_predictor_initializers_present_flag`.
    pub pps_palette_predictor_initializers_present_flag: bool,
    /// `pps_num_palette_predictor_initializers`, when present.
    pub pps_num_palette_predictor_initializers: Option<u64>,
    /// `monochrome_palette_flag`, when the initializer count is non-zero.
    pub monochrome_palette_flag: Option<bool>,
    /// `luma_bit_depth_entry_minus8`, when the initializer count is non-zero.
    pub luma_bit_depth_entry_minus8: Option<u64>,
    /// `chroma_bit_depth_entry_minus8`, when a non-monochrome initializer exists.
    pub chroma_bit_depth_entry_minus8: Option<u64>,
    /// Palette initializer values, indexed by component then entry.
    pub palette_predictor_initializers: Vec<Vec<u64>>,
}

/// One reference-layer offset entry from Annex F.7.3.2.3.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsReferenceLocationOffsetSyntax {
    /// `ref_loc_offset_layer_id`.
    pub layer_id: u8,
    /// Scaled-reference offsets, when present.
    pub scaled_ref_layer_offsets: Option<[i64; 4]>,
    /// Reference-region offsets, when present.
    pub ref_region_offsets: Option<[i64; 4]>,
    /// Resampling phase values, when present.
    pub resample_phase: Option<[u64; 4]>,
}

/// One leaf of the colour-mapping octant tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColourMappingLeafSyntax {
    /// `coded_res_flag` values in luma-part and coefficient order.
    pub coded_res_flags: Vec<bool>,
    /// `res_coeff_q` values for coded entries.
    pub res_coeff_q: Vec<[u64; 3]>,
    /// `res_coeff_r` values for coded entries.
    pub res_coeff_r: Vec<[u64; 3]>,
    /// `res_coeff_s` values for coded entries.
    pub res_coeff_s: Vec<[bool; 3]>,
}

/// Recursive colour-mapping octant syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColourMappingOctantSyntax {
    /// `inpDepth` at this node.
    pub input_depth: u8,
    /// `idxY` at this node.
    pub idx_y: u64,
    /// `idxCb` at this node.
    pub idx_cb: u64,
    /// `idxCr` at this node.
    pub idx_cr: u64,
    /// `split_octant_flag`, inferred false at the maximum depth.
    pub split_octant_flag: bool,
    /// Child octants in k/m/n order.
    pub children: Vec<ColourMappingOctantSyntax>,
    /// Leaf syntax when this node is not split.
    pub leaf: Option<ColourMappingLeafSyntax>,
}

/// Colour-mapping table syntax from Annex F.7.3.2.3.5.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColourMappingTableSyntax {
    /// `num_cm_ref_layers_minus1`.
    pub num_cm_ref_layers_minus1: u64,
    /// `cm_ref_layer_id` values.
    pub cm_ref_layer_id: Vec<u8>,
    /// `cm_octant_depth`.
    pub cm_octant_depth: u8,
    /// `cm_y_part_num_log2`.
    pub cm_y_part_num_log2: u8,
    /// Input/output luma/chroma bit-depth offsets.
    pub luma_bit_depth_cm_input_minus8: u64,
    /// Input/output luma/chroma bit-depth offsets.
    pub chroma_bit_depth_cm_input_minus8: u64,
    /// Input/output luma/chroma bit-depth offsets.
    pub luma_bit_depth_cm_output_minus8: u64,
    /// Input/output luma/chroma bit-depth offsets.
    pub chroma_bit_depth_cm_output_minus8: u64,
    /// `cm_res_quant_bits`.
    pub cm_res_quant_bits: u8,
    /// `cm_delta_flc_bits_minus1`.
    pub cm_delta_flc_bits_minus1: u8,
    /// Adaptation thresholds, present at octant depth one.
    pub cm_adapt_threshold_u_delta: Option<i64>,
    /// Adaptation thresholds, present at octant depth one.
    pub cm_adapt_threshold_v_delta: Option<i64>,
    /// Root octant syntax.
    pub root: ColourMappingOctantSyntax,
}

/// PPS multilayer extension syntax from Annex F.7.3.2.3.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsMultilayerExtensionSyntax {
    /// `poc_reset_info_present_flag`.
    pub poc_reset_info_present_flag: bool,
    /// `pps_infer_scaling_list_flag`.
    pub pps_infer_scaling_list_flag: bool,
    /// `pps_scaling_list_ref_layer_id`, when scaling-list inference is used.
    pub pps_scaling_list_ref_layer_id: Option<u8>,
    /// Reference-layer location offsets.
    pub reference_location_offsets: Vec<PpsReferenceLocationOffsetSyntax>,
    /// `colour_mapping_enabled_flag`.
    pub colour_mapping_enabled_flag: bool,
    /// Colour mapping table, when enabled.
    pub colour_mapping_table: Option<ColourMappingTableSyntax>,
}

/// Delta-DLT syntax from Annex I.7.3.2.3.8.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeltaDltSyntax {
    /// `num_val_delta_dlt`.
    pub num_val_delta_dlt: u64,
    /// `max_diff`, when more than one value is present.
    pub max_diff: Option<u64>,
    /// `min_diff_minus1`, when required.
    pub min_diff_minus1: Option<u64>,
    /// `delta_dlt_val0`, when values are present.
    pub delta_dlt_val0: Option<u64>,
    /// `delta_val_diff_minus_min` values.
    pub delta_val_diff_minus_min: Vec<u64>,
}

/// One depth-layer DLT entry from Annex I.7.3.2.3.7.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsDepthDltSyntax {
    /// `dlt_flag`.
    pub dlt_flag: bool,
    /// `dlt_pred_flag`, when present.
    pub dlt_pred_flag: Option<bool>,
    /// `dlt_val_flags_present_flag`, when present.
    pub dlt_val_flags_present_flag: Option<bool>,
    /// `dlt_value_flag` values, when explicitly signalled.
    pub dlt_value_flags: Vec<bool>,
    /// `delta_dlt()` syntax, when used.
    pub delta_dlt: Option<DeltaDltSyntax>,
}

/// PPS 3D extension syntax from Annex I.7.3.2.3.7.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pps3dExtensionSyntax {
    /// `dlts_present_flag`.
    pub dlts_present_flag: bool,
    /// `pps_depth_layers_minus1`, when DLTs are present.
    pub pps_depth_layers_minus1: Option<u8>,
    /// `pps_bit_depth_for_depth_layers_minus8`, when DLTs are present.
    pub pps_bit_depth_for_depth_layers_minus8: Option<u8>,
    /// DLT entries in depth-layer order.
    pub depth_layers: Vec<PpsDepthDltSyntax>,
}

/// PPS extension selector syntax from §7.3.2.3.1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PpsExtensionSyntax {
    /// `pps_range_extension_flag`.
    pub pps_range_extension_flag: bool,
    /// `pps_multilayer_extension_flag`.
    pub pps_multilayer_extension_flag: bool,
    /// `pps_3d_extension_flag`.
    pub pps_3d_extension_flag: bool,
    /// `pps_scc_extension_flag`.
    pub pps_scc_extension_flag: bool,
    /// `pps_extension_4bits`.
    pub pps_extension_4bits: u8,
    /// Range extension syntax, when selected.
    pub range_extension: Option<PpsRangeExtensionSyntax>,
    /// Multilayer extension syntax, when selected.
    pub multilayer_extension: Option<PpsMultilayerExtensionSyntax>,
    /// 3D extension syntax, when selected.
    pub three_d_extension: Option<Pps3dExtensionSyntax>,
    /// SCC extension syntax, when selected.
    pub scc_extension: Option<PpsSccExtensionSyntax>,
    /// `pps_extension_data_flag` values when `pps_extension_4bits` is non-zero.
    pub extension_data: Vec<bool>,
    /// Whether `rbsp_trailing_bits()` was consumed by this parser.
    pub trailing_bits_parsed: bool,
}

/// Complete picture parameter set syntax through its extension bodies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PictureParameterSetSyntax {
    /// `pps_pic_parameter_set_id`.
    pub pps_pic_parameter_set_id: u64,
    /// `pps_seq_parameter_set_id`.
    pub pps_seq_parameter_set_id: u64,
    /// `dependent_slice_segments_enabled_flag`.
    pub dependent_slice_segments_enabled_flag: bool,
    /// `output_flag_present_flag`.
    pub output_flag_present_flag: bool,
    /// `num_extra_slice_header_bits`.
    pub num_extra_slice_header_bits: u8,
    /// `sign_data_hiding_enabled_flag`.
    pub sign_data_hiding_enabled_flag: bool,
    /// `cabac_init_present_flag`.
    pub cabac_init_present_flag: bool,
    /// `num_ref_idx_l0_default_active_minus1`.
    pub num_ref_idx_l0_default_active_minus1: u64,
    /// `num_ref_idx_l1_default_active_minus1`.
    pub num_ref_idx_l1_default_active_minus1: u64,
    /// `init_qp_minus26`.
    pub init_qp_minus26: i64,
    /// `constrained_intra_pred_flag`.
    pub constrained_intra_pred_flag: bool,
    /// `transform_skip_enabled_flag`.
    pub transform_skip_enabled_flag: bool,
    /// `cu_qp_delta_enabled_flag`.
    pub cu_qp_delta_enabled_flag: bool,
    /// `diff_cu_qp_delta_depth`, when CU QP deltas are enabled.
    pub diff_cu_qp_delta_depth: Option<u64>,
    /// `pps_cb_qp_offset`.
    pub pps_cb_qp_offset: i64,
    /// `pps_cr_qp_offset`.
    pub pps_cr_qp_offset: i64,
    /// `pps_slice_chroma_qp_offsets_present_flag`.
    pub pps_slice_chroma_qp_offsets_present_flag: bool,
    /// `weighted_pred_flag`.
    pub weighted_pred_flag: bool,
    /// `weighted_bipred_flag`.
    pub weighted_bipred_flag: bool,
    /// `transquant_bypass_enabled_flag`.
    pub transquant_bypass_enabled_flag: bool,
    /// `tiles_enabled_flag`.
    pub tiles_enabled_flag: bool,
    /// `entropy_coding_sync_enabled_flag`.
    pub entropy_coding_sync_enabled_flag: bool,
    /// Tile syntax, when tiles are enabled.
    pub tiles: Option<PpsTileSyntax>,
    /// `pps_loop_filter_across_slices_enabled_flag`.
    pub pps_loop_filter_across_slices_enabled_flag: bool,
    /// Deblocking syntax, when present.
    pub deblocking_filter_control: Option<PpsDeblockingFilterSyntax>,
    /// `pps_scaling_list_data_present_flag`.
    pub pps_scaling_list_data_present_flag: bool,
    /// Scaling-list data, when present.
    pub scaling_list_data: Option<ScalingListData>,
    /// `lists_modification_present_flag`.
    pub lists_modification_present_flag: bool,
    /// `log2_parallel_merge_level_minus2`.
    pub log2_parallel_merge_level_minus2: u64,
    /// `slice_segment_header_extension_present_flag`.
    pub slice_segment_header_extension_present_flag: bool,
    /// `pps_extension_present_flag`.
    pub pps_extension_present_flag: bool,
    /// PPS extension syntax, when present.
    pub pps_extension: Option<PpsExtensionSyntax>,
}

fn bit_depth_bits(bit_depth_minus8: u64) -> Result<usize, SyntaxError> {
    let bit_depth = bit_depth_minus8
        .checked_add(8)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "palette bit depth overflows",
        ))?;
    usize::try_from(bit_depth)
        .ok()
        .filter(|&bits| bits <= 64)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "palette initializer bit depth must be at most 64",
        ))
}

impl PpsRangeExtensionSyntax {
    fn parse(
        reader: &mut BitReader<'_>,
        transform_skip_enabled_flag: bool,
    ) -> Result<Self, SyntaxError> {
        let log2_max_transform_skip_block_size_minus2 = if transform_skip_enabled_flag {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let cross_component_prediction_enabled_flag = reader.read_u(1)? != 0;
        let chroma_qp_offset_list_enabled_flag = reader.read_u(1)? != 0;
        let (
            diff_cu_chroma_qp_offset_depth,
            chroma_qp_offset_list_len_minus1,
            cb_qp_offset_list,
            cr_qp_offset_list,
        ) = if chroma_qp_offset_list_enabled_flag {
            let diff_cu_chroma_qp_offset_depth = reader.read_ue()?;
            let chroma_qp_offset_list_len_minus1 = reader.read_ue()?;
            let count = usize::try_from(chroma_qp_offset_list_len_minus1)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "too many chroma QP offset entries",
                ))?;
            let mut cb_qp_offset_list = Vec::with_capacity(count);
            let mut cr_qp_offset_list = Vec::with_capacity(count);
            for _ in 0..count {
                cb_qp_offset_list.push(reader.read_se()?);
                cr_qp_offset_list.push(reader.read_se()?);
            }
            (
                Some(diff_cu_chroma_qp_offset_depth),
                Some(chroma_qp_offset_list_len_minus1),
                cb_qp_offset_list,
                cr_qp_offset_list,
            )
        } else {
            (None, None, Vec::new(), Vec::new())
        };
        Ok(Self {
            log2_max_transform_skip_block_size_minus2,
            cross_component_prediction_enabled_flag,
            chroma_qp_offset_list_enabled_flag,
            diff_cu_chroma_qp_offset_depth,
            chroma_qp_offset_list_len_minus1,
            cb_qp_offset_list,
            cr_qp_offset_list,
            log2_sao_offset_scale_luma: reader.read_ue()?,
            log2_sao_offset_scale_chroma: reader.read_ue()?,
        })
    }
}

impl PpsSccExtensionSyntax {
    fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let pps_curr_pic_ref_enabled_flag = reader.read_u(1)? != 0;
        let residual_adaptive_colour_transform_enabled_flag = reader.read_u(1)? != 0;
        let (
            pps_slice_act_qp_offsets_present_flag,
            pps_act_y_qp_offset_plus5,
            pps_act_cb_qp_offset_plus5,
            pps_act_cr_qp_offset_plus3,
        ) = if residual_adaptive_colour_transform_enabled_flag {
            (
                Some(reader.read_u(1)? != 0),
                Some(reader.read_se()?),
                Some(reader.read_se()?),
                Some(reader.read_se()?),
            )
        } else {
            (None, None, None, None)
        };
        let pps_palette_predictor_initializers_present_flag = reader.read_u(1)? != 0;
        let (
            pps_num_palette_predictor_initializers,
            monochrome_palette_flag,
            luma_bit_depth_entry_minus8,
            chroma_bit_depth_entry_minus8,
            palette_predictor_initializers,
        ) = if pps_palette_predictor_initializers_present_flag {
            let count_value = reader.read_ue()?;
            let count = usize::try_from(count_value).map_err(|_| {
                SyntaxError::InvalidSyntaxValue("too many palette predictor initializers")
            })?;
            if count == 0 {
                (Some(count_value), None, None, None, Vec::new())
            } else {
                let monochrome_palette_flag = reader.read_u(1)? != 0;
                let luma_bit_depth_entry_minus8 = reader.read_ue()?;
                let chroma_bit_depth_entry_minus8 = if monochrome_palette_flag {
                    None
                } else {
                    Some(reader.read_ue()?)
                };
                let luma_bits = bit_depth_bits(luma_bit_depth_entry_minus8)?;
                let chroma_bits = chroma_bit_depth_entry_minus8
                    .map(bit_depth_bits)
                    .transpose()?;
                let component_count = if monochrome_palette_flag { 1 } else { 3 };
                let mut initializers = Vec::with_capacity(component_count);
                for component in 0..component_count {
                    let bit_count = if component == 0 {
                        luma_bits
                    } else {
                        chroma_bits.ok_or(SyntaxError::InvalidSyntaxValue(
                            "missing chroma palette bit depth",
                        ))?
                    };
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(reader.read_u(bit_count)?);
                    }
                    initializers.push(values);
                }
                (
                    Some(count_value),
                    Some(monochrome_palette_flag),
                    Some(luma_bit_depth_entry_minus8),
                    chroma_bit_depth_entry_minus8,
                    initializers,
                )
            }
        } else {
            (None, None, None, None, Vec::new())
        };
        Ok(Self {
            pps_curr_pic_ref_enabled_flag,
            residual_adaptive_colour_transform_enabled_flag,
            pps_slice_act_qp_offsets_present_flag,
            pps_act_y_qp_offset_plus5,
            pps_act_cb_qp_offset_plus5,
            pps_act_cr_qp_offset_plus3,
            pps_palette_predictor_initializers_present_flag,
            pps_num_palette_predictor_initializers,
            monochrome_palette_flag,
            luma_bit_depth_entry_minus8,
            chroma_bit_depth_entry_minus8,
            palette_predictor_initializers,
        })
    }
}

fn ceil_log2_u64(value: u64) -> usize {
    if value <= 1 {
        0
    } else {
        (64 - (value - 1).leading_zeros()) as usize
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_colour_mapping_octants(
    reader: &mut BitReader<'_>,
    input_depth: u8,
    idx_y: u64,
    idx_cb: u64,
    idx_cr: u64,
    input_length: u64,
    cm_octant_depth: u8,
    part_num_y: usize,
    res_coeff_r_bits: usize,
) -> Result<ColourMappingOctantSyntax, SyntaxError> {
    let split_octant_flag = if input_depth < cm_octant_depth {
        reader.read_u(1)? != 0
    } else {
        false
    };
    if split_octant_flag {
        let child_length = input_length / 2;
        let mut children = Vec::with_capacity(8);
        for k in 0..2u64 {
            for m in 0..2u64 {
                for n in 0..2u64 {
                    children.push(parse_colour_mapping_octants(
                        reader,
                        input_depth + 1,
                        idx_y + part_num_y as u64 * k * child_length,
                        idx_cb + m * child_length,
                        idx_cr + n * child_length,
                        child_length,
                        cm_octant_depth,
                        part_num_y,
                        res_coeff_r_bits,
                    )?);
                }
            }
        }
        return Ok(ColourMappingOctantSyntax {
            input_depth,
            idx_y,
            idx_cb,
            idx_cr,
            split_octant_flag,
            children,
            leaf: None,
        });
    }
    let mut coded_res_flags = Vec::with_capacity(part_num_y * 4);
    let mut res_coeff_q = Vec::new();
    let mut res_coeff_r = Vec::new();
    let mut res_coeff_s = Vec::new();
    for _ in 0..part_num_y {
        for _ in 0..4 {
            let coded = reader.read_u(1)? != 0;
            coded_res_flags.push(coded);
            if coded {
                let mut q = [0; 3];
                let mut r = [0; 3];
                let mut s = [false; 3];
                for component in 0..3 {
                    q[component] = reader.read_ue()?;
                    if res_coeff_r_bits > 0 {
                        r[component] = reader.read_u(res_coeff_r_bits)?;
                    }
                    if q[component] != 0 || r[component] != 0 {
                        s[component] = reader.read_u(1)? != 0;
                    }
                }
                res_coeff_q.push(q);
                res_coeff_r.push(r);
                res_coeff_s.push(s);
            }
        }
    }
    Ok(ColourMappingOctantSyntax {
        input_depth,
        idx_y,
        idx_cb,
        idx_cr,
        split_octant_flag,
        children: Vec::new(),
        leaf: Some(ColourMappingLeafSyntax {
            coded_res_flags,
            res_coeff_q,
            res_coeff_r,
            res_coeff_s,
        }),
    })
}

impl PpsMultilayerExtensionSyntax {
    /// Parses `pps_multilayer_extension()` and its optional colour mapping.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let poc_reset_info_present_flag = reader.read_u(1)? != 0;
        let pps_infer_scaling_list_flag = reader.read_u(1)? != 0;
        let pps_scaling_list_ref_layer_id = if pps_infer_scaling_list_flag {
            Some(reader.read_u(6)? as u8)
        } else {
            None
        };
        let num_ref_loc_offsets = reader.read_ue()?;
        let count = usize::try_from(num_ref_loc_offsets)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("too many reference locations"))?;
        let mut reference_location_offsets = Vec::with_capacity(count);
        for _ in 0..count {
            let layer_id = reader.read_u(6)? as u8;
            let scaled_ref_layer_offsets = if reader.read_u(1)? != 0 {
                Some([
                    reader.read_se()?,
                    reader.read_se()?,
                    reader.read_se()?,
                    reader.read_se()?,
                ])
            } else {
                None
            };
            let ref_region_offsets = if reader.read_u(1)? != 0 {
                Some([
                    reader.read_se()?,
                    reader.read_se()?,
                    reader.read_se()?,
                    reader.read_se()?,
                ])
            } else {
                None
            };
            let resample_phase = if reader.read_u(1)? != 0 {
                Some([
                    reader.read_ue()?,
                    reader.read_ue()?,
                    reader.read_ue()?,
                    reader.read_ue()?,
                ])
            } else {
                None
            };
            reference_location_offsets.push(PpsReferenceLocationOffsetSyntax {
                layer_id,
                scaled_ref_layer_offsets,
                ref_region_offsets,
                resample_phase,
            });
        }
        let colour_mapping_enabled_flag = reader.read_u(1)? != 0;
        let colour_mapping_table = if colour_mapping_enabled_flag {
            Some(parse_colour_mapping_table(reader)?)
        } else {
            None
        };
        Ok(Self {
            poc_reset_info_present_flag,
            pps_infer_scaling_list_flag,
            pps_scaling_list_ref_layer_id,
            reference_location_offsets,
            colour_mapping_enabled_flag,
            colour_mapping_table,
        })
    }
}

fn parse_colour_mapping_table(
    reader: &mut BitReader<'_>,
) -> Result<ColourMappingTableSyntax, SyntaxError> {
    let num_cm_ref_layers_minus1 = reader.read_ue()?;
    let reference_count = usize::try_from(num_cm_ref_layers_minus1)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "too many colour mapping layers",
        ))?;
    let mut cm_ref_layer_id = Vec::with_capacity(reference_count);
    for _ in 0..reference_count {
        cm_ref_layer_id.push(reader.read_u(6)? as u8);
    }
    let cm_octant_depth = reader.read_u(2)? as u8;
    let cm_y_part_num_log2 = reader.read_u(2)? as u8;
    let luma_bit_depth_cm_input_minus8 = reader.read_ue()?;
    let chroma_bit_depth_cm_input_minus8 = reader.read_ue()?;
    let luma_bit_depth_cm_output_minus8 = reader.read_ue()?;
    let chroma_bit_depth_cm_output_minus8 = reader.read_ue()?;
    let cm_res_quant_bits = reader.read_u(2)? as u8;
    let cm_delta_flc_bits_minus1 = reader.read_u(2)? as u8;
    let (cm_adapt_threshold_u_delta, cm_adapt_threshold_v_delta) = if cm_octant_depth == 1 {
        (Some(reader.read_se()?), Some(reader.read_se()?))
    } else {
        (None, None)
    };
    let input_luma_bits =
        luma_bit_depth_cm_input_minus8
            .checked_add(8)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "colour mapping bit depth overflows",
            ))?;
    let output_luma_bits =
        luma_bit_depth_cm_output_minus8
            .checked_add(8)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "colour mapping bit depth overflows",
            ))?;
    let cm_res_lsb = (10i64 + i64::try_from(input_luma_bits).unwrap_or(i64::MAX)
        - i64::try_from(output_luma_bits).unwrap_or(i64::MAX)
        - i64::from(cm_res_quant_bits)
        - i64::from(cm_delta_flc_bits_minus1 + 1))
    .max(0);
    let res_coeff_r_bits = usize::try_from(cm_res_lsb)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("colour mapping remainder width overflows"))?;
    let part_num_y = 1usize.checked_shl(u32::from(cm_y_part_num_log2)).ok_or(
        SyntaxError::InvalidSyntaxValue("colour mapping partition count overflows"),
    )?;
    let input_length =
        1u64.checked_shl(u32::from(cm_octant_depth))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "colour mapping input size overflows",
            ))?;
    let root = parse_colour_mapping_octants(
        reader,
        0,
        0,
        0,
        0,
        input_length,
        cm_octant_depth,
        part_num_y,
        res_coeff_r_bits,
    )?;
    Ok(ColourMappingTableSyntax {
        num_cm_ref_layers_minus1,
        cm_ref_layer_id,
        cm_octant_depth,
        cm_y_part_num_log2,
        luma_bit_depth_cm_input_minus8,
        chroma_bit_depth_cm_input_minus8,
        luma_bit_depth_cm_output_minus8,
        chroma_bit_depth_cm_output_minus8,
        cm_res_quant_bits,
        cm_delta_flc_bits_minus1,
        cm_adapt_threshold_u_delta,
        cm_adapt_threshold_v_delta,
        root,
    })
}

impl Pps3dExtensionSyntax {
    /// Parses `pps_3d_extension()` and depth look-up table syntax.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let dlts_present_flag = reader.read_u(1)? != 0;
        if !dlts_present_flag {
            return Ok(Self {
                dlts_present_flag,
                pps_depth_layers_minus1: None,
                pps_bit_depth_for_depth_layers_minus8: None,
                depth_layers: Vec::new(),
            });
        }
        let pps_depth_layers_minus1 = reader.read_u(6)? as u8;
        let pps_bit_depth_for_depth_layers_minus8 = reader.read_u(4)? as u8;
        let bit_depth = usize::from(pps_bit_depth_for_depth_layers_minus8) + 8;
        let depth_value_count = 1usize
            .checked_shl(u32::try_from(bit_depth).unwrap_or(u32::MAX))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "depth lookup range overflows",
            ))?;
        let mut depth_layers = Vec::with_capacity(usize::from(pps_depth_layers_minus1) + 1);
        for layer in 0..=pps_depth_layers_minus1 {
            let dlt_flag = reader.read_u(1)? != 0;
            if !dlt_flag {
                depth_layers.push(PpsDepthDltSyntax {
                    dlt_flag,
                    dlt_pred_flag: None,
                    dlt_val_flags_present_flag: None,
                    dlt_value_flags: Vec::new(),
                    delta_dlt: None,
                });
                continue;
            }
            let dlt_pred_value = reader.read_u(1)? != 0;
            let dlt_pred_flag = Some(dlt_pred_value);
            let dlt_val_flags_present_flag = if layer == 0 || !dlt_pred_value {
                Some(reader.read_u(1)? != 0)
            } else {
                None
            };
            let (dlt_value_flags, delta_dlt) = if dlt_val_flags_present_flag == Some(true) {
                let mut flags = Vec::with_capacity(depth_value_count);
                for _ in 0..depth_value_count {
                    flags.push(reader.read_u(1)? != 0);
                }
                (flags, None)
            } else {
                (Vec::new(), Some(parse_delta_dlt(reader, bit_depth)?))
            };
            depth_layers.push(PpsDepthDltSyntax {
                dlt_flag,
                dlt_pred_flag,
                dlt_val_flags_present_flag,
                dlt_value_flags,
                delta_dlt,
            });
        }
        Ok(Self {
            dlts_present_flag,
            pps_depth_layers_minus1: Some(pps_depth_layers_minus1),
            pps_bit_depth_for_depth_layers_minus8: Some(pps_bit_depth_for_depth_layers_minus8),
            depth_layers,
        })
    }
}

fn parse_delta_dlt(
    reader: &mut BitReader<'_>,
    bit_depth: usize,
) -> Result<DeltaDltSyntax, SyntaxError> {
    let num_val_delta_dlt = reader.read_u(bit_depth)?;
    let max_diff = if num_val_delta_dlt > 1 {
        Some(reader.read_u(bit_depth)?)
    } else {
        None
    };
    let min_diff_minus1 = if num_val_delta_dlt > 2 && max_diff.unwrap_or(0) > 0 {
        Some(reader.read_u(ceil_log2_u64(max_diff.unwrap_or(0) + 1))?)
    } else {
        None
    };
    let delta_dlt_val0 = if num_val_delta_dlt > 0 {
        Some(reader.read_u(bit_depth)?)
    } else {
        None
    };
    let min_diff = min_diff_minus1.unwrap_or_else(|| max_diff.unwrap_or(0).saturating_sub(1)) + 1;
    let diff_range = max_diff.unwrap_or(0).saturating_sub(min_diff) + 1;
    let diff_bits = ceil_log2_u64(diff_range);
    let mut delta_val_diff_minus_min = Vec::new();
    if max_diff.unwrap_or(0) > min_diff {
        for _ in 1..num_val_delta_dlt {
            delta_val_diff_minus_min.push(reader.read_u(diff_bits)?);
        }
    }
    Ok(DeltaDltSyntax {
        num_val_delta_dlt,
        max_diff,
        min_diff_minus1,
        delta_dlt_val0,
        delta_val_diff_minus_min,
    })
}

impl PpsExtensionSyntax {
    /// Parses PPS extension selectors and all selected extension bodies.
    pub fn parse(
        reader: &mut BitReader<'_>,
        transform_skip_enabled_flag: bool,
    ) -> Result<Self, SyntaxError> {
        let pps_range_extension_flag = reader.read_u(1)? != 0;
        let pps_multilayer_extension_flag = reader.read_u(1)? != 0;
        let pps_3d_extension_flag = reader.read_u(1)? != 0;
        let pps_scc_extension_flag = reader.read_u(1)? != 0;
        let pps_extension_4bits = reader.read_u(4)? as u8;
        let range_extension = if pps_range_extension_flag {
            Some(PpsRangeExtensionSyntax::parse(
                reader,
                transform_skip_enabled_flag,
            )?)
        } else {
            None
        };
        let multilayer_extension = if pps_multilayer_extension_flag {
            Some(PpsMultilayerExtensionSyntax::parse(reader)?)
        } else {
            None
        };
        let three_d_extension = if pps_3d_extension_flag {
            Some(Pps3dExtensionSyntax::parse(reader)?)
        } else {
            None
        };
        let scc_extension = if pps_scc_extension_flag {
            Some(PpsSccExtensionSyntax::parse(reader)?)
        } else {
            None
        };
        let mut extension_data = Vec::new();
        let trailing_bits_parsed = {
            if pps_extension_4bits != 0 {
                while reader.more_rbsp_data() {
                    extension_data.push(reader.read_u(1)? != 0);
                }
            }
            reader.read_rbsp_trailing_bits()?;
            true
        };
        Ok(Self {
            pps_range_extension_flag,
            pps_multilayer_extension_flag,
            pps_3d_extension_flag,
            pps_scc_extension_flag,
            pps_extension_4bits,
            range_extension,
            multilayer_extension,
            three_d_extension,
            scc_extension,
            extension_data,
            trailing_bits_parsed,
        })
    }
}

impl PictureParameterSetSyntax {
    /// Parses a complete `pic_parameter_set_rbsp()` including multilayer, 3D,
    /// SCC and extension-data syntax.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let pps_pic_parameter_set_id = reader.read_ue()?;
        let pps_seq_parameter_set_id = reader.read_ue()?;
        let dependent_slice_segments_enabled_flag = reader.read_u(1)? != 0;
        let output_flag_present_flag = reader.read_u(1)? != 0;
        let num_extra_slice_header_bits = reader.read_u(3)? as u8;
        let sign_data_hiding_enabled_flag = reader.read_u(1)? != 0;
        let cabac_init_present_flag = reader.read_u(1)? != 0;
        let num_ref_idx_l0_default_active_minus1 = reader.read_ue()?;
        let num_ref_idx_l1_default_active_minus1 = reader.read_ue()?;
        let init_qp_minus26 = reader.read_se()?;
        let constrained_intra_pred_flag = reader.read_u(1)? != 0;
        let transform_skip_enabled_flag = reader.read_u(1)? != 0;
        let cu_qp_delta_enabled_flag = reader.read_u(1)? != 0;
        let diff_cu_qp_delta_depth = if cu_qp_delta_enabled_flag {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let pps_cb_qp_offset = reader.read_se()?;
        let pps_cr_qp_offset = reader.read_se()?;
        let pps_slice_chroma_qp_offsets_present_flag = reader.read_u(1)? != 0;
        let weighted_pred_flag = reader.read_u(1)? != 0;
        let weighted_bipred_flag = reader.read_u(1)? != 0;
        let transquant_bypass_enabled_flag = reader.read_u(1)? != 0;
        let tiles_enabled_flag = reader.read_u(1)? != 0;
        let entropy_coding_sync_enabled_flag = reader.read_u(1)? != 0;
        let tiles = if tiles_enabled_flag {
            let num_tile_columns_minus1 = reader.read_ue()?;
            let num_tile_rows_minus1 = reader.read_ue()?;
            let uniform_spacing_flag = reader.read_u(1)? != 0;
            let column_width_minus1 = if uniform_spacing_flag {
                Vec::new()
            } else {
                let count = usize::try_from(num_tile_columns_minus1)
                    .map_err(|_| SyntaxError::InvalidSyntaxValue("too many tile columns"))?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(reader.read_ue()?);
                }
                values
            };
            let row_height_minus1 = if uniform_spacing_flag {
                Vec::new()
            } else {
                let count = usize::try_from(num_tile_rows_minus1)
                    .map_err(|_| SyntaxError::InvalidSyntaxValue("too many tile rows"))?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(reader.read_ue()?);
                }
                values
            };
            let loop_filter_across_tiles_enabled_flag = reader.read_u(1)? != 0;
            Some(PpsTileSyntax {
                num_tile_columns_minus1,
                num_tile_rows_minus1,
                uniform_spacing_flag,
                column_width_minus1,
                row_height_minus1,
                loop_filter_across_tiles_enabled_flag,
            })
        } else {
            None
        };
        let pps_loop_filter_across_slices_enabled_flag = reader.read_u(1)? != 0;
        let deblocking_filter_control_present_flag = reader.read_u(1)? != 0;
        let deblocking_filter_control = if deblocking_filter_control_present_flag {
            let deblocking_filter_override_enabled_flag = reader.read_u(1)? != 0;
            let pps_deblocking_filter_disabled_flag = reader.read_u(1)? != 0;
            let (pps_beta_offset_div2, pps_tc_offset_div2) = if pps_deblocking_filter_disabled_flag
            {
                (None, None)
            } else {
                (Some(reader.read_se()?), Some(reader.read_se()?))
            };
            Some(PpsDeblockingFilterSyntax {
                deblocking_filter_override_enabled_flag,
                pps_deblocking_filter_disabled_flag,
                pps_beta_offset_div2,
                pps_tc_offset_div2,
            })
        } else {
            None
        };
        let pps_scaling_list_data_present_flag = reader.read_u(1)? != 0;
        let scaling_list_data = if pps_scaling_list_data_present_flag {
            Some(ScalingListData::parse(reader)?)
        } else {
            None
        };
        let lists_modification_present_flag = reader.read_u(1)? != 0;
        let log2_parallel_merge_level_minus2 = reader.read_ue()?;
        let slice_segment_header_extension_present_flag = reader.read_u(1)? != 0;
        let pps_extension_present_flag = reader.read_u(1)? != 0;
        let pps_extension = if pps_extension_present_flag {
            Some(PpsExtensionSyntax::parse(
                reader,
                transform_skip_enabled_flag,
            )?)
        } else {
            reader.read_rbsp_trailing_bits()?;
            None
        };
        Ok(Self {
            pps_pic_parameter_set_id,
            pps_seq_parameter_set_id,
            dependent_slice_segments_enabled_flag,
            output_flag_present_flag,
            num_extra_slice_header_bits,
            sign_data_hiding_enabled_flag,
            cabac_init_present_flag,
            num_ref_idx_l0_default_active_minus1,
            num_ref_idx_l1_default_active_minus1,
            init_qp_minus26,
            constrained_intra_pred_flag,
            transform_skip_enabled_flag,
            cu_qp_delta_enabled_flag,
            diff_cu_qp_delta_depth,
            pps_cb_qp_offset,
            pps_cr_qp_offset,
            pps_slice_chroma_qp_offsets_present_flag,
            weighted_pred_flag,
            weighted_bipred_flag,
            transquant_bypass_enabled_flag,
            tiles_enabled_flag,
            entropy_coding_sync_enabled_flag,
            tiles,
            pps_loop_filter_across_slices_enabled_flag,
            deblocking_filter_control,
            pps_scaling_list_data_present_flag,
            scaling_list_data,
            lists_modification_present_flag,
            log2_parallel_merge_level_minus2,
            slice_segment_header_extension_present_flag,
            pps_extension_present_flag,
            pps_extension,
        })
    }
}
