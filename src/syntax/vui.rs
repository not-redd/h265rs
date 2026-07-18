use super::{BitReader, SyntaxError};

/// VUI timing and optional HRD syntax from Annex E.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VuiTimingInfo {
    /// `vui_num_units_in_tick`.
    pub num_units_in_tick: u32,
    /// `vui_time_scale`.
    pub time_scale: u32,
    /// `vui_poc_proportional_to_timing_flag`.
    pub poc_proportional_to_timing_flag: bool,
    /// `vui_num_ticks_poc_diff_one_minus1`, when present.
    pub num_ticks_poc_diff_one_minus1: Option<u64>,
    /// `vui_hrd_parameters_present_flag`.
    pub hrd_parameters_present_flag: bool,
    /// HRD syntax, when present.
    pub hrd_parameters: Option<HrdParameters>,
}

/// One coded-picture-buffer entry from `sub_layer_hrd_parameters`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CpbEntry {
    /// `bit_rate_value_minus1`.
    pub bit_rate_value_minus1: u64,
    /// `cpb_size_value_minus1`.
    pub cpb_size_value_minus1: u64,
    /// `cpb_size_du_value_minus1`, when sub-picture HRD is present.
    pub cpb_size_du_value_minus1: Option<u64>,
    /// `bit_rate_du_value_minus1`, when sub-picture HRD is present.
    pub bit_rate_du_value_minus1: Option<u64>,
    /// `cbr_flag`.
    pub cbr_flag: bool,
}

/// HRD entries for one temporal sub-layer and one HRD type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubLayerHrdParameters {
    /// Entries signalled by `cpb_cnt_minus1`.
    pub cpb_entries: Vec<CpbEntry>,
}

/// HRD syntax from Annex E.2.2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrdParameters {
    /// `nal_hrd_parameters_present_flag`.
    pub nal_hrd_parameters_present_flag: bool,
    /// `vcl_hrd_parameters_present_flag`.
    pub vcl_hrd_parameters_present_flag: bool,
    /// `sub_pic_hrd_params_present_flag`.
    pub sub_pic_hrd_params_present_flag: bool,
    /// `tick_divisor_minus2`, when sub-picture HRD is present.
    pub tick_divisor_minus2: Option<u8>,
    /// `du_cpb_removal_delay_increment_length_minus1`, when present.
    pub du_cpb_removal_delay_increment_length_minus1: Option<u8>,
    /// `sub_pic_cpb_params_in_pic_timing_sei_flag`, when present.
    pub sub_pic_cpb_params_in_pic_timing_sei_flag: Option<bool>,
    /// `dpb_output_delay_du_length_minus1`, when present.
    pub dpb_output_delay_du_length_minus1: Option<u8>,
    /// `bit_rate_scale`, when NAL or VCL HRD is present.
    pub bit_rate_scale: Option<u8>,
    /// `cpb_size_scale`, when NAL or VCL HRD is present.
    pub cpb_size_scale: Option<u8>,
    /// `cpb_size_du_scale`, when sub-picture HRD is present.
    pub cpb_size_du_scale: Option<u8>,
    /// `initial_cpb_removal_delay_length_minus1`, when NAL or VCL HRD is present.
    pub initial_cpb_removal_delay_length_minus1: Option<u8>,
    /// `au_cpb_removal_delay_length_minus1`, when NAL or VCL HRD is present.
    pub au_cpb_removal_delay_length_minus1: Option<u8>,
    /// `dpb_output_delay_length_minus1`, when NAL or VCL HRD is present.
    pub dpb_output_delay_length_minus1: Option<u8>,
    /// HRD syntax for each temporal sub-layer.
    pub sub_layers: Vec<HrdSubLayerParameters>,
}

/// Fixed-rate, delay, and CPB syntax for one HRD temporal sub-layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrdSubLayerParameters {
    /// `fixed_pic_rate_general_flag`.
    pub fixed_pic_rate_general_flag: bool,
    /// `fixed_pic_rate_within_cvs_flag`, or its inferred value.
    pub fixed_pic_rate_within_cvs_flag: bool,
    /// `elemental_duration_in_tc_minus1`, when present.
    pub elemental_duration_in_tc_minus1: Option<u64>,
    /// `low_delay_hrd_flag`, when present.
    pub low_delay_hrd_flag: Option<bool>,
    /// `cpb_cnt_minus1`, when present.
    pub cpb_cnt_minus1: Option<u64>,
    /// NAL HRD entries, when NAL HRD is present.
    pub nal_hrd_parameters: Option<SubLayerHrdParameters>,
    /// VCL HRD entries, when VCL HRD is present.
    pub vcl_hrd_parameters: Option<SubLayerHrdParameters>,
}

/// Bitstream-restriction syntax from `vui_parameters`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitstreamRestriction {
    /// `tiles_fixed_structure_flag`.
    pub tiles_fixed_structure_flag: bool,
    /// `motion_vectors_over_pic_boundaries_flag`.
    pub motion_vectors_over_pic_boundaries_flag: bool,
    /// `restricted_ref_pic_lists_flag`.
    pub restricted_ref_pic_lists_flag: bool,
    /// `min_spatial_segmentation_idc`.
    pub min_spatial_segmentation_idc: u64,
    /// `max_bytes_per_pic_denom`.
    pub max_bytes_per_pic_denom: u64,
    /// `max_bits_per_min_cu_denom`.
    pub max_bits_per_min_cu_denom: u64,
    /// `log2_max_mv_length_horizontal`.
    pub log2_max_mv_length_horizontal: u64,
    /// `log2_max_mv_length_vertical`.
    pub log2_max_mv_length_vertical: u64,
}

/// VUI syntax from Annex E.2.1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VuiParameters {
    /// `aspect_ratio_info_present_flag`.
    pub aspect_ratio_info_present_flag: bool,
    /// `aspect_ratio_idc`, when present.
    pub aspect_ratio_idc: Option<u8>,
    /// `sar_width`, when `aspect_ratio_idc` is `EXTENDED_SAR`.
    pub sar_width: Option<u16>,
    /// `sar_height`, when `aspect_ratio_idc` is `EXTENDED_SAR`.
    pub sar_height: Option<u16>,
    /// `overscan_info_present_flag`.
    pub overscan_info_present_flag: bool,
    /// `overscan_appropriate_flag`, when present.
    pub overscan_appropriate_flag: Option<bool>,
    /// `video_signal_type_present_flag`.
    pub video_signal_type_present_flag: bool,
    /// `video_format`, when video signal type is present.
    pub video_format: Option<u8>,
    /// `video_full_range_flag`, when video signal type is present.
    pub video_full_range_flag: Option<bool>,
    /// `colour_description_present_flag`.
    pub colour_description_present_flag: bool,
    /// `colour_primaries`, when colour description is present.
    pub colour_primaries: Option<u8>,
    /// `transfer_characteristics`, when colour description is present.
    pub transfer_characteristics: Option<u8>,
    /// `matrix_coeffs`, when colour description is present.
    pub matrix_coeffs: Option<u8>,
    /// `chroma_loc_info_present_flag`.
    pub chroma_loc_info_present_flag: bool,
    /// `chroma_sample_loc_type_top_field`, when present.
    pub chroma_sample_loc_type_top_field: Option<u64>,
    /// `chroma_sample_loc_type_bottom_field`, when present.
    pub chroma_sample_loc_type_bottom_field: Option<u64>,
    /// `neutral_chroma_indication_flag`.
    pub neutral_chroma_indication_flag: bool,
    /// `field_seq_flag`.
    pub field_seq_flag: bool,
    /// `frame_field_info_present_flag`.
    pub frame_field_info_present_flag: bool,
    /// `default_display_window_flag`.
    pub default_display_window_flag: bool,
    /// Default display offsets in left, right, top, bottom order.
    pub default_display_window: Option<[u64; 4]>,
    /// `vui_timing_info_present_flag`.
    pub vui_timing_info_present_flag: bool,
    /// Timing information, when present.
    pub timing_info: Option<VuiTimingInfo>,
    /// `bitstream_restriction_flag`.
    pub bitstream_restriction_flag: bool,
    /// Bitstream restrictions, when present.
    pub bitstream_restriction: Option<BitstreamRestriction>,
}

const EXTENDED_SAR: u8 = 255;

fn parse_sub_layer_hrd_parameters(
    reader: &mut BitReader<'_>,
    cpb_count: usize,
    sub_pic_hrd_params_present_flag: bool,
) -> Result<SubLayerHrdParameters, SyntaxError> {
    let mut cpb_entries = Vec::with_capacity(cpb_count);
    for _ in 0..cpb_count {
        cpb_entries.push(CpbEntry {
            bit_rate_value_minus1: reader.read_ue()?,
            cpb_size_value_minus1: reader.read_ue()?,
            cpb_size_du_value_minus1: if sub_pic_hrd_params_present_flag {
                Some(reader.read_ue()?)
            } else {
                None
            },
            bit_rate_du_value_minus1: if sub_pic_hrd_params_present_flag {
                Some(reader.read_ue()?)
            } else {
                None
            },
            cbr_flag: reader.read_u(1)? != 0,
        });
    }
    Ok(SubLayerHrdParameters { cpb_entries })
}

/// Parses `hrd_parameters(commonInfPresentFlag, maxNumSubLayersMinus1)`.
pub fn parse_hrd_parameters(
    reader: &mut BitReader<'_>,
    common_inf_present_flag: bool,
    max_num_sub_layers_minus1: u8,
) -> Result<HrdParameters, SyntaxError> {
    if max_num_sub_layers_minus1 > 6 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "maxNumSubLayersMinus1 must be at most 6",
        ));
    }

    let (nal_hrd_parameters_present_flag, vcl_hrd_parameters_present_flag) =
        if common_inf_present_flag {
            (reader.read_u(1)? != 0, reader.read_u(1)? != 0)
        } else {
            (false, false)
        };
    let hrd_present = nal_hrd_parameters_present_flag || vcl_hrd_parameters_present_flag;
    let mut sub_pic_hrd_params_present_flag = false;
    let mut tick_divisor_minus2 = None;
    let mut du_cpb_removal_delay_increment_length_minus1 = None;
    let mut sub_pic_cpb_params_in_pic_timing_sei_flag = None;
    let mut dpb_output_delay_du_length_minus1 = None;
    let mut bit_rate_scale = None;
    let mut cpb_size_scale = None;
    let mut cpb_size_du_scale = None;
    let mut initial_cpb_removal_delay_length_minus1 = None;
    let mut au_cpb_removal_delay_length_minus1 = None;
    let mut dpb_output_delay_length_minus1 = None;

    if hrd_present {
        sub_pic_hrd_params_present_flag = reader.read_u(1)? != 0;
        if sub_pic_hrd_params_present_flag {
            tick_divisor_minus2 = Some(reader.read_u(8)? as u8);
            du_cpb_removal_delay_increment_length_minus1 = Some(reader.read_u(5)? as u8);
            sub_pic_cpb_params_in_pic_timing_sei_flag = Some(reader.read_u(1)? != 0);
            dpb_output_delay_du_length_minus1 = Some(reader.read_u(5)? as u8);
        }
        bit_rate_scale = Some(reader.read_u(4)? as u8);
        cpb_size_scale = Some(reader.read_u(4)? as u8);
        if sub_pic_hrd_params_present_flag {
            cpb_size_du_scale = Some(reader.read_u(4)? as u8);
        }
        initial_cpb_removal_delay_length_minus1 = Some(reader.read_u(5)? as u8);
        au_cpb_removal_delay_length_minus1 = Some(reader.read_u(5)? as u8);
        dpb_output_delay_length_minus1 = Some(reader.read_u(5)? as u8);
    }

    let mut sub_layers = Vec::with_capacity(usize::from(max_num_sub_layers_minus1) + 1);
    for _ in 0..=usize::from(max_num_sub_layers_minus1) {
        let fixed_pic_rate_general_flag = reader.read_u(1)? != 0;
        let fixed_pic_rate_within_cvs_flag = if fixed_pic_rate_general_flag {
            true
        } else {
            reader.read_u(1)? != 0
        };
        let elemental_duration_in_tc_minus1 = if fixed_pic_rate_within_cvs_flag {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let low_delay_hrd_flag = if fixed_pic_rate_within_cvs_flag {
            Some(false)
        } else {
            Some(reader.read_u(1)? != 0)
        };
        let cpb_cnt_minus1 = if low_delay_hrd_flag == Some(false) {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let cpb_count = cpb_cnt_minus1
            .map(|value| {
                usize::try_from(value)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or(SyntaxError::InvalidSyntaxValue("too many CPB entries"))
            })
            .transpose()?
            .unwrap_or(0);
        let nal_hrd_parameters = if nal_hrd_parameters_present_flag {
            Some(parse_sub_layer_hrd_parameters(
                reader,
                cpb_count,
                sub_pic_hrd_params_present_flag,
            )?)
        } else {
            None
        };
        let vcl_hrd_parameters = if vcl_hrd_parameters_present_flag {
            Some(parse_sub_layer_hrd_parameters(
                reader,
                cpb_count,
                sub_pic_hrd_params_present_flag,
            )?)
        } else {
            None
        };
        sub_layers.push(HrdSubLayerParameters {
            fixed_pic_rate_general_flag,
            fixed_pic_rate_within_cvs_flag,
            elemental_duration_in_tc_minus1,
            low_delay_hrd_flag,
            cpb_cnt_minus1,
            nal_hrd_parameters,
            vcl_hrd_parameters,
        });
    }

    Ok(HrdParameters {
        nal_hrd_parameters_present_flag,
        vcl_hrd_parameters_present_flag,
        sub_pic_hrd_params_present_flag,
        tick_divisor_minus2,
        du_cpb_removal_delay_increment_length_minus1,
        sub_pic_cpb_params_in_pic_timing_sei_flag,
        dpb_output_delay_du_length_minus1,
        bit_rate_scale,
        cpb_size_scale,
        cpb_size_du_scale,
        initial_cpb_removal_delay_length_minus1,
        au_cpb_removal_delay_length_minus1,
        dpb_output_delay_length_minus1,
        sub_layers,
    })
}

/// Parses `vui_parameters()` from Annex E.2.1.
pub fn parse_vui_parameters(
    reader: &mut BitReader<'_>,
    max_num_sub_layers_minus1: u8,
) -> Result<VuiParameters, SyntaxError> {
    let aspect_ratio_info_present_flag = reader.read_u(1)? != 0;
    let (aspect_ratio_idc, sar_width, sar_height) = if aspect_ratio_info_present_flag {
        let aspect_ratio_idc = reader.read_u(8)? as u8;
        if aspect_ratio_idc == EXTENDED_SAR {
            (
                Some(aspect_ratio_idc),
                Some(reader.read_u(16)? as u16),
                Some(reader.read_u(16)? as u16),
            )
        } else {
            (Some(aspect_ratio_idc), None, None)
        }
    } else {
        (None, None, None)
    };
    let overscan_info_present_flag = reader.read_u(1)? != 0;
    let overscan_appropriate_flag = if overscan_info_present_flag {
        Some(reader.read_u(1)? != 0)
    } else {
        None
    };
    let video_signal_type_present_flag = reader.read_u(1)? != 0;
    let (
        video_format,
        video_full_range_flag,
        colour_description_present_flag,
        colour_primaries,
        transfer_characteristics,
        matrix_coeffs,
    ) = if video_signal_type_present_flag {
        let video_format = reader.read_u(3)? as u8;
        let video_full_range_flag = reader.read_u(1)? != 0;
        let colour_description_present_flag = reader.read_u(1)? != 0;
        let (colour_primaries, transfer_characteristics, matrix_coeffs) =
            if colour_description_present_flag {
                (
                    Some(reader.read_u(8)? as u8),
                    Some(reader.read_u(8)? as u8),
                    Some(reader.read_u(8)? as u8),
                )
            } else {
                (None, None, None)
            };
        (
            Some(video_format),
            Some(video_full_range_flag),
            colour_description_present_flag,
            colour_primaries,
            transfer_characteristics,
            matrix_coeffs,
        )
    } else {
        (None, None, false, None, None, None)
    };
    let chroma_loc_info_present_flag = reader.read_u(1)? != 0;
    let (chroma_sample_loc_type_top_field, chroma_sample_loc_type_bottom_field) =
        if chroma_loc_info_present_flag {
            (Some(reader.read_ue()?), Some(reader.read_ue()?))
        } else {
            (None, None)
        };
    let neutral_chroma_indication_flag = reader.read_u(1)? != 0;
    let field_seq_flag = reader.read_u(1)? != 0;
    let frame_field_info_present_flag = reader.read_u(1)? != 0;
    let default_display_window_flag = reader.read_u(1)? != 0;
    let default_display_window = if default_display_window_flag {
        Some([
            reader.read_ue()?,
            reader.read_ue()?,
            reader.read_ue()?,
            reader.read_ue()?,
        ])
    } else {
        None
    };
    let vui_timing_info_present_flag = reader.read_u(1)? != 0;
    let timing_info = if vui_timing_info_present_flag {
        let num_units_in_tick = u32::try_from(reader.read_u(32)?).map_err(|_| {
            SyntaxError::InvalidSyntaxValue("vui_num_units_in_tick does not fit u32")
        })?;
        let time_scale = u32::try_from(reader.read_u(32)?)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("vui_time_scale does not fit u32"))?;
        let poc_proportional_to_timing_flag = reader.read_u(1)? != 0;
        let num_ticks_poc_diff_one_minus1 = if poc_proportional_to_timing_flag {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let hrd_parameters_present_flag = reader.read_u(1)? != 0;
        let hrd_parameters = if hrd_parameters_present_flag {
            Some(parse_hrd_parameters(
                reader,
                true,
                max_num_sub_layers_minus1,
            )?)
        } else {
            None
        };
        Some(VuiTimingInfo {
            num_units_in_tick,
            time_scale,
            poc_proportional_to_timing_flag,
            num_ticks_poc_diff_one_minus1,
            hrd_parameters_present_flag,
            hrd_parameters,
        })
    } else {
        None
    };
    let bitstream_restriction_flag = reader.read_u(1)? != 0;
    let bitstream_restriction = if bitstream_restriction_flag {
        Some(BitstreamRestriction {
            tiles_fixed_structure_flag: reader.read_u(1)? != 0,
            motion_vectors_over_pic_boundaries_flag: reader.read_u(1)? != 0,
            restricted_ref_pic_lists_flag: reader.read_u(1)? != 0,
            min_spatial_segmentation_idc: reader.read_ue()?,
            max_bytes_per_pic_denom: reader.read_ue()?,
            max_bits_per_min_cu_denom: reader.read_ue()?,
            log2_max_mv_length_horizontal: reader.read_ue()?,
            log2_max_mv_length_vertical: reader.read_ue()?,
        })
    } else {
        None
    };
    Ok(VuiParameters {
        aspect_ratio_info_present_flag,
        aspect_ratio_idc,
        sar_width,
        sar_height,
        overscan_info_present_flag,
        overscan_appropriate_flag,
        video_signal_type_present_flag,
        video_format,
        video_full_range_flag,
        colour_description_present_flag,
        colour_primaries,
        transfer_characteristics,
        matrix_coeffs,
        chroma_loc_info_present_flag,
        chroma_sample_loc_type_top_field,
        chroma_sample_loc_type_bottom_field,
        neutral_chroma_indication_flag,
        field_seq_flag,
        frame_field_info_present_flag,
        default_display_window_flag,
        default_display_window,
        vui_timing_info_present_flag,
        timing_info,
        bitstream_restriction_flag,
        bitstream_restriction,
    })
}
