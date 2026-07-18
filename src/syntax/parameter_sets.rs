use super::{parse_profile_tier_level, BitReader, ProfileTierLevel, SyntaxError};

/// The parameter-set ordering values repeated for each sub-layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubLayerOrderingInfo {
    /// `max_dec_pic_buffering_minus1`.
    pub max_dec_pic_buffering_minus1: u64,
    /// `max_num_reorder_pics`.
    pub max_num_reorder_pics: u64,
    /// `max_latency_increase_plus1`.
    pub max_latency_increase_plus1: u64,
}

/// VPS syntax through `vps_timing_info_present_flag`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VideoParameterSetHeader {
    /// `vps_video_parameter_set_id`.
    pub vps_video_parameter_set_id: u8,
    /// `vps_base_layer_internal_flag`.
    pub vps_base_layer_internal_flag: bool,
    /// `vps_base_layer_available_flag`.
    pub vps_base_layer_available_flag: bool,
    /// `vps_max_layers_minus1`.
    pub vps_max_layers_minus1: u8,
    /// `vps_max_sub_layers_minus1`.
    pub vps_max_sub_layers_minus1: u8,
    /// `vps_temporal_id_nesting_flag`.
    pub vps_temporal_id_nesting_flag: bool,
    /// Reserved 16-bit VPS field.
    pub vps_reserved_0xffff_16bits: u16,
    /// Shared profile/tier/level syntax.
    pub profile_tier_level: ProfileTierLevel,
    /// `vps_sub_layer_ordering_info_present_flag`.
    pub vps_sub_layer_ordering_info_present_flag: bool,
    /// Ordering values in ascending sub-layer order.
    pub sub_layer_ordering_info: Vec<SubLayerOrderingInfo>,
    /// `vps_max_layer_id`.
    pub vps_max_layer_id: u8,
    /// `vps_num_layer_sets_minus1`.
    pub vps_num_layer_sets_minus1: u64,
    /// Layer-set inclusion flags, indexed by layer set then layer ID.
    pub layer_id_included_flag: Vec<Vec<bool>>,
    /// `vps_timing_info_present_flag`.
    pub vps_timing_info_present_flag: bool,
}

impl VideoParameterSetHeader {
    /// Parses the VPS through its timing-info presence flag.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let vps_video_parameter_set_id = reader.read_u(4)? as u8;
        let vps_base_layer_internal_flag = reader.read_u(1)? != 0;
        let vps_base_layer_available_flag = reader.read_u(1)? != 0;
        let vps_max_layers_minus1 = reader.read_u(6)? as u8;
        let vps_max_sub_layers_minus1 = reader.read_u(3)? as u8;
        let vps_temporal_id_nesting_flag = reader.read_u(1)? != 0;
        let vps_reserved_0xffff_16bits = reader.read_u(16)? as u16;
        let profile_tier_level = parse_profile_tier_level(reader, true, vps_max_sub_layers_minus1)?;
        let vps_sub_layer_ordering_info_present_flag = reader.read_u(1)? != 0;
        let first = if vps_sub_layer_ordering_info_present_flag {
            0
        } else {
            usize::from(vps_max_sub_layers_minus1)
        };
        let mut sub_layer_ordering_info =
            Vec::with_capacity(usize::from(vps_max_sub_layers_minus1) + 1);
        for _ in first..=usize::from(vps_max_sub_layers_minus1) {
            sub_layer_ordering_info.push(SubLayerOrderingInfo {
                max_dec_pic_buffering_minus1: reader.read_ue()?,
                max_num_reorder_pics: reader.read_ue()?,
                max_latency_increase_plus1: reader.read_ue()?,
            });
        }
        let vps_max_layer_id = reader.read_u(6)? as u8;
        let vps_num_layer_sets_minus1 = reader.read_ue()?;
        let mut layer_id_included_flag = Vec::with_capacity(vps_num_layer_sets_minus1 as usize);
        for _ in 1..=vps_num_layer_sets_minus1 {
            let mut layer_set = Vec::with_capacity(usize::from(vps_max_layer_id) + 1);
            for _ in 0..=vps_max_layer_id {
                layer_set.push(reader.read_u(1)? != 0);
            }
            layer_id_included_flag.push(layer_set);
        }
        let vps_timing_info_present_flag = reader.read_u(1)? != 0;
        Ok(Self {
            vps_video_parameter_set_id,
            vps_base_layer_internal_flag,
            vps_base_layer_available_flag,
            vps_max_layers_minus1,
            vps_max_sub_layers_minus1,
            vps_temporal_id_nesting_flag,
            vps_reserved_0xffff_16bits,
            profile_tier_level,
            vps_sub_layer_ordering_info_present_flag,
            sub_layer_ordering_info,
            vps_max_layer_id,
            vps_num_layer_sets_minus1,
            layer_id_included_flag,
            vps_timing_info_present_flag,
        })
    }
}

/// SPS syntax through `max_transform_hierarchy_depth_intra`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceParameterSetHeader {
    /// `sps_video_parameter_set_id`.
    pub sps_video_parameter_set_id: u8,
    /// `sps_max_sub_layers_minus1`.
    pub sps_max_sub_layers_minus1: u8,
    /// `sps_temporal_id_nesting_flag`.
    pub sps_temporal_id_nesting_flag: bool,
    /// Shared profile/tier/level syntax.
    pub profile_tier_level: ProfileTierLevel,
    /// `sps_seq_parameter_set_id`.
    pub sps_seq_parameter_set_id: u64,
    /// `chroma_format_idc`.
    pub chroma_format_idc: u64,
    /// `separate_colour_plane_flag`, when `chroma_format_idc == 3`.
    pub separate_colour_plane_flag: bool,
    /// `pic_width_in_luma_samples`.
    pub pic_width_in_luma_samples: u64,
    /// `pic_height_in_luma_samples`.
    pub pic_height_in_luma_samples: u64,
    /// `conformance_window_flag`.
    pub conformance_window_flag: bool,
    /// Conformance offsets in left, right, top, bottom order.
    pub conformance_window: Option<[u64; 4]>,
    /// `bit_depth_luma_minus8`.
    pub bit_depth_luma_minus8: u64,
    /// `bit_depth_chroma_minus8`.
    pub bit_depth_chroma_minus8: u64,
    /// `log2_max_pic_order_cnt_lsb_minus4`.
    pub log2_max_pic_order_cnt_lsb_minus4: u64,
    /// `sps_sub_layer_ordering_info_present_flag`.
    pub sps_sub_layer_ordering_info_present_flag: bool,
    /// Ordering values in ascending sub-layer order.
    pub sub_layer_ordering_info: Vec<SubLayerOrderingInfo>,
    /// `log2_min_luma_coding_block_size_minus3`.
    pub log2_min_luma_coding_block_size_minus3: u64,
    /// `log2_diff_max_min_luma_coding_block_size`.
    pub log2_diff_max_min_luma_coding_block_size: u64,
    /// `log2_min_luma_transform_block_size_minus2`.
    pub log2_min_luma_transform_block_size_minus2: u64,
    /// `log2_diff_max_min_luma_transform_block_size`.
    pub log2_diff_max_min_luma_transform_block_size: u64,
    /// `max_transform_hierarchy_depth_inter`.
    pub max_transform_hierarchy_depth_inter: u64,
    /// `max_transform_hierarchy_depth_intra`.
    pub max_transform_hierarchy_depth_intra: u64,
}

impl SequenceParameterSetHeader {
    /// Parses the SPS common header through
    /// `max_transform_hierarchy_depth_intra`, leaving the following
    /// `scaling_list_enabled_flag` unread.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let sps_video_parameter_set_id = reader.read_u(4)? as u8;
        let sps_max_sub_layers_minus1 = reader.read_u(3)? as u8;
        let sps_temporal_id_nesting_flag = reader.read_u(1)? != 0;
        let profile_tier_level = parse_profile_tier_level(reader, true, sps_max_sub_layers_minus1)?;
        let sps_seq_parameter_set_id = reader.read_ue()?;
        let chroma_format_idc = reader.read_ue()?;
        let separate_colour_plane_flag = if chroma_format_idc == 3 {
            reader.read_u(1)? != 0
        } else {
            false
        };
        let pic_width_in_luma_samples = reader.read_ue()?;
        let pic_height_in_luma_samples = reader.read_ue()?;
        let conformance_window_flag = reader.read_u(1)? != 0;
        let conformance_window = if conformance_window_flag {
            Some([
                reader.read_ue()?,
                reader.read_ue()?,
                reader.read_ue()?,
                reader.read_ue()?,
            ])
        } else {
            None
        };
        let bit_depth_luma_minus8 = reader.read_ue()?;
        let bit_depth_chroma_minus8 = reader.read_ue()?;
        let log2_max_pic_order_cnt_lsb_minus4 = reader.read_ue()?;
        let sps_sub_layer_ordering_info_present_flag = reader.read_u(1)? != 0;
        let first = if sps_sub_layer_ordering_info_present_flag {
            0
        } else {
            usize::from(sps_max_sub_layers_minus1)
        };
        let mut sub_layer_ordering_info =
            Vec::with_capacity(usize::from(sps_max_sub_layers_minus1) + 1);
        for _ in first..=usize::from(sps_max_sub_layers_minus1) {
            sub_layer_ordering_info.push(SubLayerOrderingInfo {
                max_dec_pic_buffering_minus1: reader.read_ue()?,
                max_num_reorder_pics: reader.read_ue()?,
                max_latency_increase_plus1: reader.read_ue()?,
            });
        }
        let log2_min_luma_coding_block_size_minus3 = reader.read_ue()?;
        let log2_diff_max_min_luma_coding_block_size = reader.read_ue()?;
        let log2_min_luma_transform_block_size_minus2 = reader.read_ue()?;
        let log2_diff_max_min_luma_transform_block_size = reader.read_ue()?;
        let max_transform_hierarchy_depth_inter = reader.read_ue()?;
        let max_transform_hierarchy_depth_intra = reader.read_ue()?;
        Ok(Self {
            sps_video_parameter_set_id,
            sps_max_sub_layers_minus1,
            sps_temporal_id_nesting_flag,
            profile_tier_level,
            sps_seq_parameter_set_id,
            chroma_format_idc,
            separate_colour_plane_flag,
            pic_width_in_luma_samples,
            pic_height_in_luma_samples,
            conformance_window_flag,
            conformance_window,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            log2_max_pic_order_cnt_lsb_minus4,
            sps_sub_layer_ordering_info_present_flag,
            sub_layer_ordering_info,
            log2_min_luma_coding_block_size_minus3,
            log2_diff_max_min_luma_coding_block_size,
            log2_min_luma_transform_block_size_minus2,
            log2_diff_max_min_luma_transform_block_size,
            max_transform_hierarchy_depth_inter,
            max_transform_hierarchy_depth_intra,
        })
    }
}
