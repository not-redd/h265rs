use super::{
    parse_short_term_reference_picture_set, BitReader, ShortTermReferencePictureSet, SyntaxError,
};

/// Context required by the general slice-segment header syntax.
#[derive(Clone, Debug)]
pub struct SliceSegmentHeaderContext<'a> {
    /// NAL unit type, used for IRAP and IDR conditions.
    pub nal_unit_type: u8,
    /// Number of bits used by `slice_segment_address`.
    pub slice_segment_address_bits: usize,
    /// `dependent_slice_segments_enabled_flag`.
    pub dependent_slice_segments_enabled_flag: bool,
    /// `output_flag_present_flag`.
    pub output_flag_present_flag: bool,
    /// `num_extra_slice_header_bits`.
    pub num_extra_slice_header_bits: usize,
    /// `separate_colour_plane_flag`.
    pub separate_colour_plane_flag: bool,
    /// `log2_max_pic_order_cnt_lsb_minus4`.
    pub log2_max_pic_order_cnt_lsb_minus4: u64,
    /// SPS short-term RPS entries.
    pub short_term_ref_pic_sets: &'a [ShortTermReferencePictureSet],
    /// `long_term_ref_pics_present_flag`.
    pub long_term_ref_pics_present_flag: bool,
    /// `num_long_term_ref_pics_sps`.
    pub num_long_term_ref_pics_sps: u64,
    /// `sps_temporal_mvp_enabled_flag`.
    pub sps_temporal_mvp_enabled_flag: bool,
    /// `sample_adaptive_offset_enabled_flag`.
    pub sample_adaptive_offset_enabled_flag: bool,
    /// Whether the chroma array type is non-monochrome.
    pub chroma_array_type_nonzero: bool,
    /// `num_ref_idx_l0_default_active_minus1`.
    pub num_ref_idx_l0_default_active_minus1: u64,
    /// `num_ref_idx_l1_default_active_minus1`.
    pub num_ref_idx_l1_default_active_minus1: u64,
    /// `lists_modification_present_flag`.
    pub lists_modification_present_flag: bool,
    /// `num_pic_total_curr` used by reference-list modification syntax.
    pub num_pic_total_curr: usize,
    /// `weighted_pred_flag`.
    pub weighted_pred_flag: bool,
    /// `weighted_bipred_flag`.
    pub weighted_bipred_flag: bool,
    /// `cabac_init_present_flag`.
    pub cabac_init_present_flag: bool,
    /// `pps_slice_chroma_qp_offsets_present_flag`.
    pub pps_slice_chroma_qp_offsets_present_flag: bool,
    /// `pps_slice_act_qp_offsets_present_flag`.
    pub pps_slice_act_qp_offsets_present_flag: bool,
    /// `chroma_qp_offset_list_enabled_flag`.
    pub chroma_qp_offset_list_enabled_flag: bool,
    /// `deblocking_filter_override_enabled_flag`.
    pub deblocking_filter_override_enabled_flag: bool,
    /// `pps_deblocking_filter_disabled_flag`.
    pub pps_deblocking_filter_disabled_flag: bool,
    /// `pps_loop_filter_across_slices_enabled_flag`.
    pub pps_loop_filter_across_slices_enabled_flag: bool,
    /// `tiles_enabled_flag`.
    pub tiles_enabled_flag: bool,
    /// `entropy_coding_sync_enabled_flag`.
    pub entropy_coding_sync_enabled_flag: bool,
    /// `slice_segment_header_extension_present_flag`.
    pub slice_segment_header_extension_present_flag: bool,
    /// `motion_vector_resolution_control_idc`.
    pub motion_vector_resolution_control_idc: u8,
    /// Flags indicating whether each active reference is the current picture.
    pub l0_reference_is_current: &'a [bool],
    /// Flags indicating whether each active reference is the current picture.
    pub l1_reference_is_current: &'a [bool],
}

/// Slice long-term reference-picture syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceLongTermReferencePicture {
    /// `lt_idx_sps`, when referring to an SPS candidate.
    pub lt_idx_sps: Option<u64>,
    /// `poc_lsb_lt`, when signalled directly in the slice.
    pub poc_lsb_lt: Option<u64>,
    /// `used_by_curr_pic_lt_flag`, for a directly signalled candidate.
    pub used_by_curr_pic_lt_flag: Option<bool>,
    /// `delta_poc_msb_present_flag`.
    pub delta_poc_msb_present_flag: bool,
    /// `delta_poc_msb_cycle_lt`, when present.
    pub delta_poc_msb_cycle_lt: Option<u64>,
}

/// Reference-picture syntax in a slice-segment header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceReferencePictureSet {
    /// `short_term_ref_pic_set_sps_flag`.
    pub short_term_ref_pic_set_sps_flag: bool,
    /// `short_term_ref_pic_set_idx`, when an SPS RPS is selected.
    pub short_term_ref_pic_set_idx: Option<u64>,
    /// Slice-created short-term RPS, when selected.
    pub short_term_ref_pic_set: Option<ShortTermReferencePictureSet>,
    /// `num_long_term_sps`, when SPS long-term candidates exist.
    pub num_long_term_sps: Option<u64>,
    /// `num_long_term_pics`.
    pub num_long_term_pics: u64,
    /// Long-term entries in syntax order.
    pub long_term_pictures: Vec<SliceLongTermReferencePicture>,
}

/// SAO flags in a slice-segment header.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SliceSaoSyntax {
    /// `slice_sao_luma_flag`.
    pub luma_flag: bool,
    /// `slice_sao_chroma_flag`, when chroma exists.
    pub chroma_flag: Option<bool>,
}

/// Reference-list modification syntax from §7.3.6.2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceReferenceListModification {
    /// `ref_pic_modification_flag_l0`.
    pub modification_flag_l0: bool,
    /// `list_entry_l0` values.
    pub list_entry_l0: Vec<u64>,
    /// `ref_pic_modification_flag_l1`, for B slices.
    pub modification_flag_l1: Option<bool>,
    /// `list_entry_l1` values.
    pub list_entry_l1: Vec<u64>,
}

/// One side of weighted prediction syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceWeightList {
    /// Luma weight-present flags.
    pub luma_weight_flags: Vec<bool>,
    /// Chroma weight-present flags.
    pub chroma_weight_flags: Vec<bool>,
    /// Luma weight deltas, empty when the corresponding flag is false.
    pub delta_luma_weight: Vec<Option<i64>>,
    /// Luma offsets, empty when the corresponding flag is false.
    pub luma_offset: Vec<Option<i64>>,
    /// Chroma weight/offset pairs for Cb and Cr.
    pub chroma_weight_offset: Vec<[Option<(i64, i64)>; 2]>,
}

/// Weighted prediction syntax from §7.3.6.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlicePredictionWeightTable {
    /// `luma_log2_weight_denom`.
    pub luma_log2_weight_denom: u64,
    /// `delta_chroma_log2_weight_denom`, when chroma exists.
    pub delta_chroma_log2_weight_denom: Option<i64>,
    /// L0 weights.
    pub list0: SliceWeightList,
    /// L1 weights, for B slices.
    pub list1: Option<SliceWeightList>,
}

/// Parsed general slice-segment header through SAO and temporal-MVP syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SliceSegmentHeaderSyntax {
    /// `first_slice_segment_in_pic_flag`.
    pub first_slice_segment_in_pic_flag: bool,
    /// `no_output_of_prior_pics_flag`, for IRAP NAL units.
    pub no_output_of_prior_pics_flag: Option<bool>,
    /// `slice_pic_parameter_set_id`.
    pub slice_pic_parameter_set_id: u64,
    /// `dependent_slice_segment_flag`, when the slice is not first.
    pub dependent_slice_segment_flag: bool,
    /// `slice_segment_address`, when the slice is not first.
    pub slice_segment_address: Option<u64>,
    /// `slice_reserved_flag` values.
    pub slice_reserved_flags: Vec<bool>,
    /// `slice_type`, when this is not a dependent segment.
    pub slice_type: Option<u64>,
    /// `pic_output_flag`, when present.
    pub pic_output_flag: Option<bool>,
    /// `colour_plane_id`, when separate colour planes are used.
    pub colour_plane_id: Option<u8>,
    /// `slice_pic_order_cnt_lsb`, for non-IDR pictures.
    pub slice_pic_order_cnt_lsb: Option<u64>,
    /// Reference-picture syntax, for non-IDR pictures.
    pub reference_picture_set: Option<SliceReferencePictureSet>,
    /// `slice_temporal_mvp_enabled_flag`, when present.
    pub slice_temporal_mvp_enabled_flag: Option<bool>,
    /// SAO syntax, when enabled by the SPS.
    pub sao: Option<SliceSaoSyntax>,
    /// `num_ref_idx_active_override_flag`, for P/B slices.
    pub num_ref_idx_active_override_flag: Option<bool>,
    /// Effective `num_ref_idx_l0_active_minus1`, for P/B slices.
    pub num_ref_idx_l0_active_minus1: Option<u64>,
    /// Effective `num_ref_idx_l1_active_minus1`, for B slices.
    pub num_ref_idx_l1_active_minus1: Option<u64>,
    /// Reference-list modification syntax, when present.
    pub reference_list_modification: Option<SliceReferenceListModification>,
    /// `mvd_l1_zero_flag`, for B slices.
    pub mvd_l1_zero_flag: Option<bool>,
    /// `cabac_init_flag`, for P/B slices when present in the PPS.
    pub cabac_init_flag: Option<bool>,
    /// `collocated_from_l0_flag`, when temporal MVP is enabled for B slices.
    pub collocated_from_l0_flag: Option<bool>,
    /// `collocated_ref_idx`, when present.
    pub collocated_ref_idx: Option<u64>,
    /// Weighted prediction syntax, when enabled.
    pub prediction_weight_table: Option<SlicePredictionWeightTable>,
    /// `five_minus_max_num_merge_cand`, for P/B slices.
    pub five_minus_max_num_merge_cand: Option<u64>,
    /// `use_integer_mv_flag`, when motion-vector resolution control is 2.
    pub use_integer_mv_flag: Option<bool>,
    /// `slice_qp_delta`.
    pub slice_qp_delta: Option<i64>,
    /// `slice_cb_qp_offset`, when present.
    pub slice_cb_qp_offset: Option<i64>,
    /// `slice_cr_qp_offset`, when present.
    pub slice_cr_qp_offset: Option<i64>,
    /// ACT QP offsets, when present.
    pub slice_act_qp_offsets: Option<[i64; 3]>,
    /// `cu_chroma_qp_offset_enabled_flag`, when present.
    pub cu_chroma_qp_offset_enabled_flag: Option<bool>,
    /// `deblocking_filter_override_flag`, when present.
    pub deblocking_filter_override_flag: Option<bool>,
    /// `slice_deblocking_filter_disabled_flag`, when present or inferred.
    pub slice_deblocking_filter_disabled_flag: bool,
    /// Slice beta and tc offsets, when deblocking is enabled.
    pub slice_deblocking_offsets: Option<[i64; 2]>,
    /// `slice_loop_filter_across_slices_enabled_flag`, when present.
    pub slice_loop_filter_across_slices_enabled_flag: Option<bool>,
    /// `num_entry_point_offsets`.
    pub num_entry_point_offsets: Option<u64>,
    /// `offset_len_minus1`, when entry points are present.
    pub offset_len_minus1: Option<u64>,
    /// `entry_point_offset_minus1` values.
    pub entry_point_offset_minus1: Vec<u64>,
    /// Slice header extension bytes.
    pub slice_segment_header_extension_data: Vec<u8>,
}

fn ceil_log2(value: usize) -> usize {
    (usize::BITS - value.saturating_sub(1).leading_zeros()) as usize
}

fn is_irap(nal_unit_type: u8) -> bool {
    (16..=23).contains(&nal_unit_type)
}

fn is_idr(nal_unit_type: u8) -> bool {
    nal_unit_type == 19 || nal_unit_type == 20
}

fn poc_lsb_bits(log2_max_pic_order_cnt_lsb_minus4: u64) -> Result<usize, SyntaxError> {
    let bits =
        log2_max_pic_order_cnt_lsb_minus4
            .checked_add(4)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "POC LSB bit width overflows",
            ))?;
    usize::try_from(bits)
        .ok()
        .filter(|&count| count <= 64)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "POC LSB bit width must be at most 64",
        ))
}

fn parse_reference_picture_set(
    reader: &mut BitReader<'_>,
    context: &SliceSegmentHeaderContext<'_>,
    poc_bits: usize,
) -> Result<SliceReferencePictureSet, SyntaxError> {
    let num_short_term_ref_pic_sets = context.short_term_ref_pic_sets.len();
    let short_term_ref_pic_set_sps_flag = if num_short_term_ref_pic_sets == 0 {
        false
    } else {
        reader.read_u(1)? != 0
    };
    let (short_term_ref_pic_set_idx, short_term_ref_pic_set) = if short_term_ref_pic_set_sps_flag {
        let index = if num_short_term_ref_pic_sets > 1 {
            Some(reader.read_u(ceil_log2(num_short_term_ref_pic_sets))?)
        } else {
            None
        };
        (index, None)
    } else {
        let set = parse_short_term_reference_picture_set(
            reader,
            num_short_term_ref_pic_sets,
            context.short_term_ref_pic_sets,
            num_short_term_ref_pic_sets,
        )?;
        (None, Some(set))
    };
    let (num_long_term_sps, num_long_term_pics, long_term_pictures) = if context
        .long_term_ref_pics_present_flag
    {
        let num_long_term_sps = if context.num_long_term_ref_pics_sps > 0 {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let num_long_term_pics = reader.read_ue()?;
        let total = num_long_term_sps
            .unwrap_or(0)
            .checked_add(num_long_term_pics)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "too many long-term slice references",
            ))?;
        let total = usize::try_from(total)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("too many long-term slice references"))?;
        let lt_idx_bits = if context.num_long_term_ref_pics_sps > 1 {
            Some(ceil_log2(
                usize::try_from(context.num_long_term_ref_pics_sps).map_err(|_| {
                    SyntaxError::InvalidSyntaxValue("too many SPS long-term references")
                })?,
            ))
        } else {
            None
        };
        let num_long_term_sps_value = num_long_term_sps.unwrap_or(0);
        if num_long_term_sps_value > context.num_long_term_ref_pics_sps {
            return Err(SyntaxError::InvalidSyntaxValue(
                "slice long-term SPS reference count exceeds SPS count",
            ));
        }
        let mut long_term_pictures = Vec::with_capacity(total);
        for index in 0..total {
            let is_sps_reference =
                u64::try_from(index).unwrap_or(u64::MAX) < num_long_term_sps_value;
            let (lt_idx_sps, poc_lsb_lt, used_by_curr_pic_lt_flag) = if is_sps_reference {
                (Some(reader.read_u(lt_idx_bits.unwrap_or(0))?), None, None)
            } else {
                (
                    None,
                    Some(reader.read_u(poc_bits)?),
                    Some(reader.read_u(1)? != 0),
                )
            };
            let delta_poc_msb_present_flag = reader.read_u(1)? != 0;
            let delta_poc_msb_cycle_lt = if delta_poc_msb_present_flag {
                Some(reader.read_ue()?)
            } else {
                None
            };
            long_term_pictures.push(SliceLongTermReferencePicture {
                lt_idx_sps,
                poc_lsb_lt,
                used_by_curr_pic_lt_flag,
                delta_poc_msb_present_flag,
                delta_poc_msb_cycle_lt,
            });
        }
        (num_long_term_sps, num_long_term_pics, long_term_pictures)
    } else {
        (None, 0, Vec::new())
    };
    Ok(SliceReferencePictureSet {
        short_term_ref_pic_set_sps_flag,
        short_term_ref_pic_set_idx,
        short_term_ref_pic_set,
        num_long_term_sps,
        num_long_term_pics,
        long_term_pictures,
    })
}

fn active_count(minus1: u64, message: &'static str) -> Result<usize, SyntaxError> {
    usize::try_from(minus1)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(SyntaxError::InvalidSyntaxValue(message))
}

fn parse_reference_list_modification(
    reader: &mut BitReader<'_>,
    slice_type: u64,
    num_ref_idx_l0_active_minus1: u64,
    num_ref_idx_l1_active_minus1: u64,
    num_pic_total_curr: usize,
) -> Result<SliceReferenceListModification, SyntaxError> {
    let entry_bits = ceil_log2(num_pic_total_curr);
    let modification_flag_l0 = reader.read_u(1)? != 0;
    let list_entry_l0 = if modification_flag_l0 {
        let count = active_count(
            num_ref_idx_l0_active_minus1,
            "too many L0 reference-list entries",
        )?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            entries.push(reader.read_u(entry_bits)?);
        }
        entries
    } else {
        Vec::new()
    };
    let (modification_flag_l1, list_entry_l1) = if slice_type == 0 {
        let flag = reader.read_u(1)? != 0;
        let entries = if flag {
            let count = active_count(
                num_ref_idx_l1_active_minus1,
                "too many L1 reference-list entries",
            )?;
            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                entries.push(reader.read_u(entry_bits)?);
            }
            entries
        } else {
            Vec::new()
        };
        (Some(flag), entries)
    } else {
        (None, Vec::new())
    };
    Ok(SliceReferenceListModification {
        modification_flag_l0,
        list_entry_l0,
        modification_flag_l1,
        list_entry_l1,
    })
}

fn parse_weight_list(
    reader: &mut BitReader<'_>,
    count: usize,
    chroma_present: bool,
    references_current: &[bool],
) -> Result<SliceWeightList, SyntaxError> {
    let mut luma_weight_flags = Vec::with_capacity(count);
    for index in 0..count {
        luma_weight_flags.push(if references_current.get(index).copied().unwrap_or(false) {
            false
        } else {
            reader.read_u(1)? != 0
        });
    }
    let mut chroma_weight_flags = Vec::with_capacity(count);
    if chroma_present {
        for index in 0..count {
            chroma_weight_flags.push(if references_current.get(index).copied().unwrap_or(false) {
                false
            } else {
                reader.read_u(1)? != 0
            });
        }
    } else {
        chroma_weight_flags.resize(count, false);
    }
    let mut delta_luma_weight = vec![None; count];
    let mut luma_offset = vec![None; count];
    let mut chroma_weight_offset = vec![[None, None]; count];
    for index in 0..count {
        if luma_weight_flags[index] {
            delta_luma_weight[index] = Some(reader.read_se()?);
            luma_offset[index] = Some(reader.read_se()?);
        }
        if chroma_weight_flags[index] {
            for offset in &mut chroma_weight_offset[index] {
                *offset = Some((reader.read_se()?, reader.read_se()?));
            }
        }
    }
    Ok(SliceWeightList {
        luma_weight_flags,
        chroma_weight_flags,
        delta_luma_weight,
        luma_offset,
        chroma_weight_offset,
    })
}

fn parse_prediction_weight_table(
    reader: &mut BitReader<'_>,
    slice_type: u64,
    num_ref_idx_l0_active_minus1: u64,
    num_ref_idx_l1_active_minus1: u64,
    chroma_present: bool,
    l0_reference_is_current: &[bool],
    l1_reference_is_current: &[bool],
) -> Result<SlicePredictionWeightTable, SyntaxError> {
    let luma_log2_weight_denom = reader.read_ue()?;
    let delta_chroma_log2_weight_denom = if chroma_present {
        Some(reader.read_se()?)
    } else {
        None
    };
    let list0 = parse_weight_list(
        reader,
        active_count(
            num_ref_idx_l0_active_minus1,
            "too many L0 weighted-prediction entries",
        )?,
        chroma_present,
        l0_reference_is_current,
    )?;
    let list1 = if slice_type == 0 {
        Some(parse_weight_list(
            reader,
            active_count(
                num_ref_idx_l1_active_minus1,
                "too many L1 weighted-prediction entries",
            )?,
            chroma_present,
            l1_reference_is_current,
        )?)
    } else {
        None
    };
    Ok(SlicePredictionWeightTable {
        luma_log2_weight_denom,
        delta_chroma_log2_weight_denom,
        list0,
        list1,
    })
}

type SliceHeaderTail = (Option<u64>, Option<u64>, Vec<u64>, Vec<u8>);

fn parse_slice_header_tail(
    reader: &mut BitReader<'_>,
    context: &SliceSegmentHeaderContext<'_>,
) -> Result<SliceHeaderTail, SyntaxError> {
    let (num_entry_point_offsets, offset_len_minus1, entry_point_offset_minus1) =
        if context.tiles_enabled_flag || context.entropy_coding_sync_enabled_flag {
            let count_value = reader.read_ue()?;
            let count = usize::try_from(count_value)
                .map_err(|_| SyntaxError::InvalidSyntaxValue("too many entry-point offsets"))?;
            if count > 0 {
                let offset_len_minus1 = reader.read_ue()?;
                let bit_count = usize::try_from(offset_len_minus1)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .filter(|&value| value <= 64)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "entry-point offset width must be at most 64",
                    ))?;
                let mut offsets = Vec::with_capacity(count);
                for _ in 0..count {
                    offsets.push(reader.read_u(bit_count)?);
                }
                (Some(count_value), Some(offset_len_minus1), offsets)
            } else {
                (Some(count_value), None, Vec::new())
            }
        } else {
            (None, None, Vec::new())
        };
    let slice_segment_header_extension_data = if context.slice_segment_header_extension_present_flag
    {
        let length = usize::try_from(reader.read_ue()?)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("slice header extension is too large"))?;
        let mut data = Vec::with_capacity(length);
        for _ in 0..length {
            data.push(reader.read_u(8)? as u8);
        }
        data
    } else {
        Vec::new()
    };
    reader.read_byte_alignment()?;
    Ok((
        num_entry_point_offsets,
        offset_len_minus1,
        entry_point_offset_minus1,
        slice_segment_header_extension_data,
    ))
}

impl<'a> SliceSegmentHeaderContext<'a> {
    /// Creates a context with the caller-supplied SPS/PPS syntax conditions.
    #[must_use]
    pub const fn new(
        nal_unit_type: u8,
        slice_segment_address_bits: usize,
        short_term_ref_pic_sets: &'a [ShortTermReferencePictureSet],
    ) -> Self {
        Self {
            nal_unit_type,
            slice_segment_address_bits,
            dependent_slice_segments_enabled_flag: false,
            output_flag_present_flag: false,
            num_extra_slice_header_bits: 0,
            separate_colour_plane_flag: false,
            log2_max_pic_order_cnt_lsb_minus4: 0,
            short_term_ref_pic_sets,
            long_term_ref_pics_present_flag: false,
            num_long_term_ref_pics_sps: 0,
            sps_temporal_mvp_enabled_flag: false,
            sample_adaptive_offset_enabled_flag: false,
            chroma_array_type_nonzero: true,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            lists_modification_present_flag: false,
            num_pic_total_curr: 0,
            weighted_pred_flag: false,
            weighted_bipred_flag: false,
            cabac_init_present_flag: false,
            pps_slice_chroma_qp_offsets_present_flag: false,
            pps_slice_act_qp_offsets_present_flag: false,
            chroma_qp_offset_list_enabled_flag: false,
            deblocking_filter_override_enabled_flag: false,
            pps_deblocking_filter_disabled_flag: false,
            pps_loop_filter_across_slices_enabled_flag: false,
            tiles_enabled_flag: false,
            entropy_coding_sync_enabled_flag: false,
            slice_segment_header_extension_present_flag: false,
            motion_vector_resolution_control_idc: 0,
            l0_reference_is_current: &[],
            l1_reference_is_current: &[],
        }
    }
}

/// Parses `slice_segment_header()` through SAO syntax.
pub fn parse_slice_segment_header(
    reader: &mut BitReader<'_>,
    context: &SliceSegmentHeaderContext<'_>,
) -> Result<SliceSegmentHeaderSyntax, SyntaxError> {
    let first_slice_segment_in_pic_flag = reader.read_u(1)? != 0;
    let no_output_of_prior_pics_flag = if is_irap(context.nal_unit_type) {
        Some(reader.read_u(1)? != 0)
    } else {
        None
    };
    let slice_pic_parameter_set_id = reader.read_ue()?;
    let (dependent_slice_segment_flag, slice_segment_address) = if first_slice_segment_in_pic_flag {
        (false, None)
    } else {
        let dependent = if context.dependent_slice_segments_enabled_flag {
            reader.read_u(1)? != 0
        } else {
            false
        };
        let address = Some(reader.read_u(context.slice_segment_address_bits)?);
        (dependent, address)
    };
    if dependent_slice_segment_flag {
        let (
            num_entry_point_offsets,
            offset_len_minus1,
            entry_point_offset_minus1,
            slice_segment_header_extension_data,
        ) = parse_slice_header_tail(reader, context)?;
        return Ok(SliceSegmentHeaderSyntax {
            first_slice_segment_in_pic_flag,
            no_output_of_prior_pics_flag,
            slice_pic_parameter_set_id,
            dependent_slice_segment_flag,
            slice_segment_address,
            slice_reserved_flags: Vec::new(),
            slice_type: None,
            pic_output_flag: None,
            colour_plane_id: None,
            slice_pic_order_cnt_lsb: None,
            reference_picture_set: None,
            slice_temporal_mvp_enabled_flag: None,
            sao: None,
            num_ref_idx_active_override_flag: None,
            num_ref_idx_l0_active_minus1: None,
            num_ref_idx_l1_active_minus1: None,
            reference_list_modification: None,
            mvd_l1_zero_flag: None,
            cabac_init_flag: None,
            collocated_from_l0_flag: None,
            collocated_ref_idx: None,
            prediction_weight_table: None,
            five_minus_max_num_merge_cand: None,
            use_integer_mv_flag: None,
            slice_qp_delta: None,
            slice_cb_qp_offset: None,
            slice_cr_qp_offset: None,
            slice_act_qp_offsets: None,
            cu_chroma_qp_offset_enabled_flag: None,
            deblocking_filter_override_flag: None,
            slice_deblocking_filter_disabled_flag: context.pps_deblocking_filter_disabled_flag,
            slice_deblocking_offsets: None,
            slice_loop_filter_across_slices_enabled_flag: None,
            num_entry_point_offsets,
            offset_len_minus1,
            entry_point_offset_minus1,
            slice_segment_header_extension_data,
        });
    }
    let mut slice_reserved_flags = Vec::with_capacity(context.num_extra_slice_header_bits);
    for _ in 0..context.num_extra_slice_header_bits {
        slice_reserved_flags.push(reader.read_u(1)? != 0);
    }
    let slice_type = reader.read_ue()?;
    let pic_output_flag = if context.output_flag_present_flag {
        Some(reader.read_u(1)? != 0)
    } else {
        None
    };
    let colour_plane_id = if context.separate_colour_plane_flag {
        Some(reader.read_u(2)? as u8)
    } else {
        None
    };
    let (slice_pic_order_cnt_lsb, reference_picture_set, slice_temporal_mvp_enabled_flag) =
        if is_idr(context.nal_unit_type) {
            (None, None, None)
        } else {
            let poc_bits = poc_lsb_bits(context.log2_max_pic_order_cnt_lsb_minus4)?;
            let slice_pic_order_cnt_lsb = Some(reader.read_u(poc_bits)?);
            let reference_picture_set =
                Some(parse_reference_picture_set(reader, context, poc_bits)?);
            let slice_temporal_mvp_enabled_flag = if context.sps_temporal_mvp_enabled_flag {
                Some(reader.read_u(1)? != 0)
            } else {
                None
            };
            (
                slice_pic_order_cnt_lsb,
                reference_picture_set,
                slice_temporal_mvp_enabled_flag,
            )
        };
    let sao = if context.sample_adaptive_offset_enabled_flag {
        let luma_flag = reader.read_u(1)? != 0;
        let chroma_flag = if context.chroma_array_type_nonzero {
            Some(reader.read_u(1)? != 0)
        } else {
            None
        };
        Some(SliceSaoSyntax {
            luma_flag,
            chroma_flag,
        })
    } else {
        None
    };
    let is_b_slice = slice_type == 0;
    let is_inter_slice = is_b_slice || slice_type == 1;
    if slice_type > 2 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "slice_type must be B, P, or I",
        ));
    }
    let (
        num_ref_idx_active_override_flag,
        num_ref_idx_l0_active_minus1,
        num_ref_idx_l1_active_minus1,
        reference_list_modification,
        mvd_l1_zero_flag,
        cabac_init_flag,
        collocated_from_l0_flag,
        collocated_ref_idx,
        prediction_weight_table,
        five_minus_max_num_merge_cand,
        use_integer_mv_flag,
    ) = if is_inter_slice {
        let override_flag = reader.read_u(1)? != 0;
        let l0 = if override_flag {
            reader.read_ue()?
        } else {
            context.num_ref_idx_l0_default_active_minus1
        };
        let l1 = if is_b_slice {
            Some(if override_flag {
                reader.read_ue()?
            } else {
                context.num_ref_idx_l1_default_active_minus1
            })
        } else {
            None
        };
        let reference_list_modification =
            if context.lists_modification_present_flag && context.num_pic_total_curr > 1 {
                Some(parse_reference_list_modification(
                    reader,
                    slice_type,
                    l0,
                    l1.unwrap_or(0),
                    context.num_pic_total_curr,
                )?)
            } else {
                None
            };
        let mvd_l1_zero_flag = if is_b_slice {
            Some(reader.read_u(1)? != 0)
        } else {
            None
        };
        let cabac_init_flag = if context.cabac_init_present_flag {
            Some(reader.read_u(1)? != 0)
        } else {
            None
        };
        let temporal_mvp = slice_temporal_mvp_enabled_flag.unwrap_or(false);
        let collocated_from_l0_flag = if temporal_mvp && is_b_slice {
            Some(reader.read_u(1)? != 0)
        } else if temporal_mvp {
            Some(true)
        } else {
            None
        };
        let collocated_from_l0 = collocated_from_l0_flag.unwrap_or(true);
        let selected_ref_count = if collocated_from_l0 {
            l0
        } else {
            l1.unwrap_or(0)
        };
        let collocated_ref_idx = if temporal_mvp && selected_ref_count > 0 {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let prediction_weight_table = if (context.weighted_pred_flag && slice_type == 1)
            || (context.weighted_bipred_flag && is_b_slice)
        {
            Some(parse_prediction_weight_table(
                reader,
                slice_type,
                l0,
                l1.unwrap_or(0),
                context.chroma_array_type_nonzero,
                context.l0_reference_is_current,
                context.l1_reference_is_current,
            )?)
        } else {
            None
        };
        let five_minus_max_num_merge_cand = Some(reader.read_ue()?);
        let use_integer_mv_flag = if context.motion_vector_resolution_control_idc == 2 {
            Some(reader.read_u(1)? != 0)
        } else {
            None
        };
        (
            Some(override_flag),
            Some(l0),
            l1,
            reference_list_modification,
            mvd_l1_zero_flag,
            cabac_init_flag,
            collocated_from_l0_flag,
            collocated_ref_idx,
            prediction_weight_table,
            five_minus_max_num_merge_cand,
            use_integer_mv_flag,
        )
    } else {
        (
            None, None, None, None, None, None, None, None, None, None, None,
        )
    };
    let slice_qp_delta = Some(reader.read_se()?);
    let slice_cb_qp_offset = if context.pps_slice_chroma_qp_offsets_present_flag {
        Some(reader.read_se()?)
    } else {
        None
    };
    let slice_cr_qp_offset = if context.pps_slice_chroma_qp_offsets_present_flag {
        Some(reader.read_se()?)
    } else {
        None
    };
    let slice_act_qp_offsets = if context.pps_slice_act_qp_offsets_present_flag {
        Some([reader.read_se()?, reader.read_se()?, reader.read_se()?])
    } else {
        None
    };
    let cu_chroma_qp_offset_enabled_flag = if context.chroma_qp_offset_list_enabled_flag {
        Some(reader.read_u(1)? != 0)
    } else {
        None
    };
    let deblocking_filter_override_flag = if context.deblocking_filter_override_enabled_flag {
        Some(reader.read_u(1)? != 0)
    } else {
        None
    };
    let slice_deblocking_filter_disabled_flag = if deblocking_filter_override_flag == Some(true) {
        reader.read_u(1)? != 0
    } else {
        context.pps_deblocking_filter_disabled_flag
    };
    let slice_deblocking_offsets = if !slice_deblocking_filter_disabled_flag {
        if deblocking_filter_override_flag == Some(true) {
            Some([reader.read_se()?, reader.read_se()?])
        } else {
            None
        }
    } else {
        None
    };
    let slice_sao_luma_flag = sao.is_some_and(|value| value.luma_flag);
    let slice_sao_chroma_flag = sao.is_some_and(|value| value.chroma_flag == Some(true));
    let slice_loop_filter_across_slices_enabled_flag = if context
        .pps_loop_filter_across_slices_enabled_flag
        && (slice_sao_luma_flag || slice_sao_chroma_flag || !slice_deblocking_filter_disabled_flag)
    {
        Some(reader.read_u(1)? != 0)
    } else {
        None
    };
    let (num_entry_point_offsets, offset_len_minus1, entry_point_offset_minus1) =
        if context.tiles_enabled_flag || context.entropy_coding_sync_enabled_flag {
            let count_value = reader.read_ue()?;
            let count = usize::try_from(count_value)
                .map_err(|_| SyntaxError::InvalidSyntaxValue("too many entry-point offsets"))?;
            if count > 0 {
                let offset_len_minus1 = reader.read_ue()?;
                let bit_count = usize::try_from(offset_len_minus1)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .filter(|&value| value <= 64)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "entry-point offset width must be at most 64",
                    ))?;
                let mut offsets = Vec::with_capacity(count);
                for _ in 0..count {
                    offsets.push(reader.read_u(bit_count)?);
                }
                (Some(count_value), Some(offset_len_minus1), offsets)
            } else {
                (Some(count_value), None, Vec::new())
            }
        } else {
            (None, None, Vec::new())
        };
    let slice_segment_header_extension_data = if context.slice_segment_header_extension_present_flag
    {
        let length = usize::try_from(reader.read_ue()?)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("slice header extension is too large"))?;
        let mut data = Vec::with_capacity(length);
        for _ in 0..length {
            data.push(reader.read_u(8)? as u8);
        }
        data
    } else {
        Vec::new()
    };
    reader.read_byte_alignment()?;
    Ok(SliceSegmentHeaderSyntax {
        first_slice_segment_in_pic_flag,
        no_output_of_prior_pics_flag,
        slice_pic_parameter_set_id,
        dependent_slice_segment_flag,
        slice_segment_address,
        slice_reserved_flags,
        slice_type: Some(slice_type),
        pic_output_flag,
        colour_plane_id,
        slice_pic_order_cnt_lsb,
        reference_picture_set,
        slice_temporal_mvp_enabled_flag,
        sao,
        num_ref_idx_active_override_flag,
        num_ref_idx_l0_active_minus1,
        num_ref_idx_l1_active_minus1,
        reference_list_modification,
        mvd_l1_zero_flag,
        cabac_init_flag,
        collocated_from_l0_flag,
        collocated_ref_idx,
        prediction_weight_table,
        five_minus_max_num_merge_cand,
        use_integer_mv_flag,
        slice_qp_delta,
        slice_cb_qp_offset,
        slice_cr_qp_offset,
        slice_act_qp_offsets,
        cu_chroma_qp_offset_enabled_flag,
        deblocking_filter_override_flag,
        slice_deblocking_filter_disabled_flag,
        slice_deblocking_offsets,
        slice_loop_filter_across_slices_enabled_flag,
        num_entry_point_offsets,
        offset_len_minus1,
        entry_point_offset_minus1,
        slice_segment_header_extension_data,
    })
}
