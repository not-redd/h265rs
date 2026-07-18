#![allow(missing_docs)]

use h265rs::{
    ebsp_to_rbsp, parse_cross_component_prediction, parse_motion_vector_difference, parse_sao,
    BitReader, CabacReader, ChromaQpOffsetState, DeltaQpState, NalUnitHeader, ParsedNalUnit,
    PictureParameterSetSyntax, ScalingListData, SequenceParameterSetHeader,
    SequenceParameterSetSyntax, SyntaxDescriptor, SyntaxError, SyntaxValue,
    VideoParameterSetSyntax,
};

#[derive(Debug)]
struct MockCabac {
    values: std::collections::VecDeque<u64>,
}

impl MockCabac {
    fn new(values: impl IntoIterator<Item = u64>) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }
}

impl CabacReader for MockCabac {
    fn read_ae(&mut self) -> Result<u64, SyntaxError> {
        self.values.pop_front().ok_or(SyntaxError::UnexpectedEnd {
            requested: 1,
            remaining: 0,
        })
    }

    fn byte_alignment(&mut self) -> Result<(), SyntaxError> {
        Ok(())
    }

    fn rbsp_slice_segment_trailing_bits(&mut self) -> Result<usize, SyntaxError> {
        Ok(0)
    }
}

fn pack_bits(bits: &str) -> Vec<u8> {
    let mut bytes = vec![0u8; bits.len().div_ceil(8)];
    for (index, bit) in bits.bytes().enumerate() {
        if bit == b'1' {
            bytes[index / 8] |= 1 << (7 - index % 8);
        }
    }
    bytes
}

fn push_bits(bits: &mut Vec<bool>, value: u64, count: usize) {
    for index in (0..count).rev() {
        bits.push(((value >> index) & 1) != 0);
    }
}

fn push_ue(bits: &mut Vec<bool>, value: u64) {
    let code_num = value + 1;
    let width = 64 - code_num.leading_zeros() as usize;
    bits.extend(std::iter::repeat_n(false, width - 1));
    push_bits(bits, code_num, width);
}

fn push_se(bits: &mut Vec<bool>, value: i64) {
    let code_num = if value > 0 {
        (value as u64) * 2 - 1
    } else {
        (-value as u64) * 2
    };
    push_ue(bits, code_num);
}

fn finish_bits(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0; bits.len().div_ceil(8)];
    for (index, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[index / 8] |= 1 << (7 - index % 8);
        }
    }
    bytes
}

fn push_profile_tier_level(bits: &mut Vec<bool>) {
    push_bits(bits, 0, 2); // general_profile_space
    push_bits(bits, 0, 1); // general_tier_flag
    push_bits(bits, 1, 5); // general_profile_idc
    push_bits(bits, 0, 32); // compatibility flags
    push_bits(bits, 0, 4); // source and frame flags
    push_bits(bits, 0, 44); // constraint flags and reserved bits
    push_bits(bits, 120, 8); // general_level_idc
}

#[test]
fn bit_reader_is_msb_first_and_peek_does_not_advance() {
    let mut reader = BitReader::new(&[0b1011_1010]);
    assert_eq!(reader.next_bits(4), Ok(0b1011));
    assert_eq!(reader.position(), 0);
    assert_eq!(reader.read_u(4), Ok(0b1011));
    assert_eq!(reader.read_i(4), Ok(-6));
    assert_eq!(reader.bits_remaining(), 0);
}

#[test]
fn exp_golomb_descriptors_decode_unsigned_and_signed_values() {
    let unsigned_bits = pack_bits("10100110");
    let mut unsigned = BitReader::new(&unsigned_bits);
    assert_eq!(unsigned.read_ue(), Ok(0));
    assert_eq!(unsigned.read_ue(), Ok(1));
    assert_eq!(unsigned.read_ue(), Ok(2));

    let signed_bits = pack_bits("101001100100");
    let mut signed = BitReader::new(&signed_bits);
    assert_eq!(signed.read_se(), Ok(0));
    assert_eq!(signed.read_se(), Ok(1));
    assert_eq!(signed.read_se(), Ok(-1));
    assert_eq!(signed.read_se(), Ok(2));
}

#[test]
fn descriptor_dispatch_matches_clause_7_2() {
    let mut reader = BitReader::new(&[0b1010_0000, b'o', b'k', 0]);
    assert_eq!(
        reader.read_descriptor(SyntaxDescriptor::Fixed(3)),
        Ok(SyntaxValue::Unsigned(0b101))
    );
    assert_eq!(
        reader.read_descriptor(SyntaxDescriptor::Unsigned(1)),
        Ok(SyntaxValue::Unsigned(0))
    );
    assert_eq!(
        reader.read_descriptor(SyntaxDescriptor::String),
        Err(SyntaxError::NotByteAligned)
    );

    let mut string_reader = BitReader::new(b"ok\0");
    assert_eq!(
        string_reader.read_descriptor(SyntaxDescriptor::String),
        Ok(SyntaxValue::String("ok".to_owned()))
    );
    let mut arithmetic_reader = BitReader::new(&[0]);
    assert_eq!(
        arithmetic_reader.read_descriptor(SyntaxDescriptor::Arithmetic),
        Err(SyntaxError::ArithmeticCodingUnsupported)
    );
}

#[test]
fn rbsp_trailing_bits_and_more_data_are_recognized() {
    let bits = pack_bits("10110000");
    let mut reader = BitReader::new(&bits);
    assert!(reader.more_rbsp_data());
    assert_eq!(reader.read_u(3), Ok(0b101));
    assert!(reader.more_data_in_payload(3));
    assert!(!reader.more_rbsp_data());
    assert_eq!(reader.read_rbsp_trailing_bits(), Ok(()));
    assert_eq!(reader.bits_remaining(), 0);
    assert!(!reader.more_data_in_payload(8));

    let only_trailing = BitReader::new(&[0b1000_0000]);
    assert!(!only_trailing.more_rbsp_data());
}

#[test]
fn nal_header_and_emulation_prevention_are_parsed() {
    let header = NalUnitHeader::parse(&[0x40, 0x01]).unwrap();
    assert_eq!(header.nal_unit_type, 32);
    assert_eq!(header.nuh_layer_id, 0);
    assert_eq!(header.nuh_temporal_id_plus1, 1);

    assert_eq!(ebsp_to_rbsp(&[0, 0, 3, 1, 2]), vec![0, 0, 1, 2]);
    let parsed = ParsedNalUnit::parse(&[0x40, 0x01, 0, 0, 3, 1]).unwrap();
    assert_eq!(parsed.header, header);
    assert_eq!(parsed.rbsp, vec![0, 0, 1]);
    assert!(matches!(
        NalUnitHeader::parse(&[0, 0]),
        Err(SyntaxError::InvalidNalHeader(_))
    ));
}

#[test]
fn vps_header_parser_follows_7_3_2_1() {
    let mut bits = Vec::new();
    push_bits(&mut bits, 3, 4); // vps_video_parameter_set_id
    push_bits(&mut bits, 1, 1); // base layer internal
    push_bits(&mut bits, 0, 1); // base layer available
    push_bits(&mut bits, 0, 6); // max layers minus one
    push_bits(&mut bits, 0, 3); // max sub-layers minus one
    push_bits(&mut bits, 1, 1); // temporal nesting
    push_bits(&mut bits, 0xffff, 16);
    push_profile_tier_level(&mut bits);
    push_bits(&mut bits, 1, 1); // ordering info present
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_bits(&mut bits, 0, 6); // max layer id
    push_ue(&mut bits, 0); // layer sets minus one
    push_bits(&mut bits, 0, 1); // timing info absent
    push_bits(&mut bits, 0, 1); // vps_extension_flag
    push_bits(&mut bits, 1, 1); // rbsp_stop_one_bit
    push_bits(&mut bits, 0, 7); // rbsp_alignment_zero_bit

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let vps = VideoParameterSetSyntax::parse(&mut reader).unwrap();
    assert_eq!(vps.header.vps_video_parameter_set_id, 3);
    assert_eq!(vps.header.vps_reserved_0xffff_16bits, 0xffff);
    assert_eq!(vps.header.profile_tier_level.general_level_idc, 120);
    assert_eq!(vps.header.sub_layer_ordering_info.len(), 1);
    assert!(!vps.header.vps_timing_info_present_flag);
    assert!(!vps.vps_extension_flag);
}

#[test]
fn sps_header_parser_derives_core_picture_fields() {
    let mut bits = Vec::new();
    push_bits(&mut bits, 0, 4); // SPS VPS id
    push_bits(&mut bits, 0, 3); // max sub-layers minus one
    push_bits(&mut bits, 1, 1); // temporal nesting
    push_profile_tier_level(&mut bits);
    push_ue(&mut bits, 2); // SPS id
    push_ue(&mut bits, 1); // 4:2:0
    push_ue(&mut bits, 1920);
    push_ue(&mut bits, 1080);
    push_bits(&mut bits, 0, 1); // no conformance window
    push_ue(&mut bits, 2); // luma depth minus eight
    push_ue(&mut bits, 2); // chroma depth minus eight
    push_ue(&mut bits, 4); // POC LSB minus four
    push_bits(&mut bits, 1, 1); // ordering info present
    for _ in 0..3 {
        push_ue(&mut bits, 0);
    }
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 3);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 3);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let sps = SequenceParameterSetHeader::parse(&mut reader).unwrap();
    assert_eq!(sps.sps_seq_parameter_set_id, 2);
    assert_eq!(sps.chroma_format_idc, 1);
    assert_eq!(sps.pic_width_in_luma_samples, 1920);
    assert_eq!(sps.pic_height_in_luma_samples, 1080);
    assert_eq!(sps.bit_depth_luma_minus8, 2);
    assert_eq!(sps.sub_layer_ordering_info.len(), 1);
}

#[test]
fn scaling_list_parser_preserves_predicted_and_derived_matrices() {
    let mut bits = Vec::new();
    push_bits(&mut bits, 1, 1); // first matrix: explicit coefficients
    for _ in 0..16 {
        push_ue(&mut bits, 0); // se(v) zero
    }
    for _ in 1..20 {
        push_bits(&mut bits, 0, 1); // prediction mode
        push_ue(&mut bits, 0); // matrix delta
    }
    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let scaling = ScalingListData::parse(&mut reader).unwrap();
    assert_eq!(scaling.matrices.len(), 20);
    assert_eq!(scaling.matrices[0].coefficients, vec![8; 16]);
    assert_eq!(scaling.matrices[0].delta_coefficients, vec![0; 16]);
    assert!(!scaling.matrices[1].pred_mode_flag);
    assert_eq!(scaling.matrices[19].size_id, 3);
    assert_eq!(scaling.matrices[19].matrix_id, 3);
}

#[test]
fn extended_sps_parser_reads_scaling_tools_and_pcm_boundary() {
    let mut bits = Vec::new();
    push_bits(&mut bits, 0, 4);
    push_bits(&mut bits, 0, 3);
    push_bits(&mut bits, 1, 1);
    push_profile_tier_level(&mut bits);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 1);
    push_ue(&mut bits, 64);
    push_ue(&mut bits, 64);
    push_bits(&mut bits, 0, 1);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_bits(&mut bits, 1, 1);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 0);
    push_bits(&mut bits, 0, 1); // scaling list disabled
    push_bits(&mut bits, 1, 1); // AMP enabled
    push_bits(&mut bits, 1, 1); // SAO enabled
    push_bits(&mut bits, 1, 1); // PCM enabled
    push_bits(&mut bits, 7, 4);
    push_bits(&mut bits, 7, 4);
    push_ue(&mut bits, 0);
    push_ue(&mut bits, 1);
    push_bits(&mut bits, 1, 1);
    push_ue(&mut bits, 1); // num_short_term_ref_pic_sets
    push_ue(&mut bits, 1); // num_negative_pics
    push_ue(&mut bits, 0); // num_positive_pics
    push_ue(&mut bits, 0); // delta_poc_s0_minus1
    push_bits(&mut bits, 1, 1); // used_by_curr_pic_s0_flag
    push_bits(&mut bits, 1, 1); // long_term_ref_pics_present_flag
    push_ue(&mut bits, 1); // num_long_term_ref_pics_sps
    push_bits(&mut bits, 5, 4); // lt_ref_pic_poc_lsb_sps
    push_bits(&mut bits, 1, 1); // used_by_curr_pic_lt_sps_flag
    push_bits(&mut bits, 1, 1); // sps_temporal_mvp_enabled_flag
    push_bits(&mut bits, 0, 1); // strong_intra_smoothing_enabled_flag
    push_bits(&mut bits, 0, 1); // vui_parameters_present_flag
    push_bits(&mut bits, 1, 1); // sps_extension_present_flag
    push_bits(&mut bits, 1, 1); // sps_range_extension_flag
    push_bits(&mut bits, 0, 1); // sps_multilayer_extension_flag
    push_bits(&mut bits, 0, 1); // sps_3d_extension_flag
    push_bits(&mut bits, 0, 1); // sps_scc_extension_flag
    push_bits(&mut bits, 0, 4); // sps_extension_4bits
    push_bits(&mut bits, 1, 1); // transform_skip_rotation_enabled_flag
    push_bits(&mut bits, 0, 1); // transform_skip_context_enabled_flag
    push_bits(&mut bits, 1, 1); // implicit_rdpcm_enabled_flag
    push_bits(&mut bits, 0, 1); // explicit_rdpcm_enabled_flag
    push_bits(&mut bits, 1, 1); // extended_precision_processing_flag
    push_bits(&mut bits, 0, 1); // intra_smoothing_disabled_flag
    push_bits(&mut bits, 1, 1); // high_precision_offsets_enabled_flag
    push_bits(&mut bits, 0, 1); // persistent_rice_adaptation_enabled_flag
    push_bits(&mut bits, 1, 1); // cabac_bypass_alignment_enabled_flag
    push_bits(&mut bits, 1, 1); // rbsp_stop_one_bit
    push_bits(&mut bits, 0, 3); // rbsp_alignment_zero_bit

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let sps = SequenceParameterSetSyntax::parse(&mut reader).unwrap();
    assert_eq!(sps.header.pic_width_in_luma_samples, 64);
    assert!(!sps.scaling_list_enabled_flag);
    assert!(sps.amp_enabled_flag);
    assert!(sps.sample_adaptive_offset_enabled_flag);
    assert_eq!(sps.pcm.unwrap().sample_bit_depth_luma_minus1, 7);
    assert_eq!(sps.num_short_term_ref_pic_sets, 1);
    assert_eq!(sps.short_term_ref_pic_sets[0].num_delta_pocs, 1);
    assert!(sps.long_term_ref_pics_present_flag);
    assert_eq!(
        sps.long_term_ref_pic_set.as_ref().unwrap().poc_lsb_sps,
        vec![5]
    );
    assert!(sps.sps_temporal_mvp_enabled_flag);
    assert!(!sps.strong_intra_smoothing_enabled_flag);
    assert!(!sps.vui_parameters_present_flag);
    assert!(sps.vui_parameters.is_none());
    assert!(sps.sps_extension_present_flag);
    let extension = sps.sps_extension.as_ref().unwrap();
    assert!(extension.sps_range_extension_flag);
    assert!(!extension.sps_scc_extension_flag);
    assert!(
        extension
            .range_extension
            .as_ref()
            .unwrap()
            .transform_skip_rotation_enabled_flag
    );
    assert!(extension.trailing_bits_parsed);
}

#[test]
fn inter_predicted_short_term_rps_uses_previous_num_delta_pocs() {
    let mut bits = Vec::new();
    push_ue(&mut bits, 1); // first set: one negative picture
    push_ue(&mut bits, 0); // no positive pictures
    push_ue(&mut bits, 0);
    push_bits(&mut bits, 1, 1);
    push_bits(&mut bits, 1, 1); // inter_ref_pic_set_prediction_flag
    push_bits(&mut bits, 0, 1); // delta_rps_sign
    push_ue(&mut bits, 0); // abs_delta_rps_minus1
    push_bits(&mut bits, 1, 1); // used_by_curr_pic_flag[0]
    push_bits(&mut bits, 0, 1); // used_by_curr_pic_flag[1]
    push_bits(&mut bits, 1, 1); // use_delta_flag[1]

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let first = h265rs::parse_short_term_reference_picture_set(&mut reader, 0, &[], 2).unwrap();
    let second = h265rs::parse_short_term_reference_picture_set(
        &mut reader,
        1,
        std::slice::from_ref(&first),
        2,
    )
    .unwrap();
    assert_eq!(second.reference_rps_idx, Some(0));
    assert_eq!(second.used_by_curr_pic_flag, vec![true, false]);
    assert_eq!(second.use_delta_flag, vec![false, true]);
    assert_eq!(second.num_delta_pocs, 2);
}

#[test]
fn vui_parser_reads_annex_e_parameters_and_hrd() {
    let mut bits = Vec::new();
    push_bits(&mut bits, 1, 1); // aspect_ratio_info_present_flag
    push_bits(&mut bits, 255, 8); // EXTENDED_SAR
    push_bits(&mut bits, 100, 16); // sar_width
    push_bits(&mut bits, 200, 16); // sar_height
    push_bits(&mut bits, 1, 1); // overscan_info_present_flag
    push_bits(&mut bits, 1, 1); // overscan_appropriate_flag
    push_bits(&mut bits, 1, 1); // video_signal_type_present_flag
    push_bits(&mut bits, 5, 3); // video_format
    push_bits(&mut bits, 1, 1); // video_full_range_flag
    push_bits(&mut bits, 1, 1); // colour_description_present_flag
    push_bits(&mut bits, 1, 8); // colour_primaries
    push_bits(&mut bits, 16, 8); // transfer_characteristics
    push_bits(&mut bits, 9, 8); // matrix_coeffs
    push_bits(&mut bits, 1, 1); // chroma_loc_info_present_flag
    push_ue(&mut bits, 2); // chroma_sample_loc_type_top_field
    push_ue(&mut bits, 3); // chroma_sample_loc_type_bottom_field
    push_bits(&mut bits, 1, 1); // neutral_chroma_indication_flag
    push_bits(&mut bits, 0, 1); // field_seq_flag
    push_bits(&mut bits, 1, 1); // frame_field_info_present_flag
    push_bits(&mut bits, 1, 1); // default_display_window_flag
    push_ue(&mut bits, 1);
    push_ue(&mut bits, 2);
    push_ue(&mut bits, 3);
    push_ue(&mut bits, 4);
    push_bits(&mut bits, 1, 1); // vui_timing_info_present_flag
    push_bits(&mut bits, 1000, 32); // vui_num_units_in_tick
    push_bits(&mut bits, 60_000, 32); // vui_time_scale
    push_bits(&mut bits, 1, 1); // vui_poc_proportional_to_timing_flag
    push_ue(&mut bits, 0); // vui_num_ticks_poc_diff_one_minus1
    push_bits(&mut bits, 1, 1); // vui_hrd_parameters_present_flag
    push_bits(&mut bits, 1, 1); // nal_hrd_parameters_present_flag
    push_bits(&mut bits, 0, 1); // vcl_hrd_parameters_present_flag
    push_bits(&mut bits, 0, 1); // sub_pic_hrd_params_present_flag
    push_bits(&mut bits, 2, 4); // bit_rate_scale
    push_bits(&mut bits, 3, 4); // cpb_size_scale
    push_bits(&mut bits, 5, 5); // initial_cpb_removal_delay_length_minus1
    push_bits(&mut bits, 4, 5); // au_cpb_removal_delay_length_minus1
    push_bits(&mut bits, 5, 5); // dpb_output_delay_length_minus1
    push_bits(&mut bits, 0, 1); // fixed_pic_rate_general_flag
    push_bits(&mut bits, 1, 1); // fixed_pic_rate_within_cvs_flag
    push_ue(&mut bits, 2); // elemental_duration_in_tc_minus1
    push_ue(&mut bits, 0); // cpb_cnt_minus1
    push_ue(&mut bits, 0); // bit_rate_value_minus1
    push_ue(&mut bits, 1); // cpb_size_value_minus1
    push_bits(&mut bits, 1, 1); // cbr_flag
    push_bits(&mut bits, 1, 1); // bitstream_restriction_flag
    push_bits(&mut bits, 1, 1); // tiles_fixed_structure_flag
    push_bits(&mut bits, 1, 1); // motion_vectors_over_pic_boundaries_flag
    push_bits(&mut bits, 0, 1); // restricted_ref_pic_lists_flag
    push_ue(&mut bits, 1); // min_spatial_segmentation_idc
    push_ue(&mut bits, 2); // max_bytes_per_pic_denom
    push_ue(&mut bits, 3); // max_bits_per_min_cu_denom
    push_ue(&mut bits, 4); // log2_max_mv_length_horizontal
    push_ue(&mut bits, 5); // log2_max_mv_length_vertical

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let vui = h265rs::parse_vui_parameters(&mut reader, 0).unwrap();
    assert_eq!(vui.aspect_ratio_idc, Some(255));
    assert_eq!(vui.sar_width, Some(100));
    assert_eq!(vui.sar_height, Some(200));
    assert_eq!(vui.video_format, Some(5));
    assert_eq!(vui.chroma_sample_loc_type_bottom_field, Some(3));
    assert_eq!(vui.default_display_window, Some([1, 2, 3, 4]));
    let timing = vui.timing_info.as_ref().unwrap();
    assert_eq!(timing.time_scale, 60_000);
    let hrd = timing.hrd_parameters.as_ref().unwrap();
    assert_eq!(hrd.sub_layers.len(), 1);
    assert_eq!(hrd.sub_layers[0].cpb_cnt_minus1, Some(0));
    assert_eq!(
        hrd.sub_layers[0]
            .nal_hrd_parameters
            .as_ref()
            .unwrap()
            .cpb_entries[0]
            .cpb_size_value_minus1,
        1
    );
    assert_eq!(
        vui.bitstream_restriction
            .unwrap()
            .log2_max_mv_length_vertical,
        5
    );
}

#[test]
fn pps_parser_reads_general_tiles_range_and_scc_syntax() {
    let mut bits = Vec::new();
    push_ue(&mut bits, 3); // pps_pic_parameter_set_id
    push_ue(&mut bits, 1); // pps_seq_parameter_set_id
    push_bits(&mut bits, 1, 1); // dependent_slice_segments_enabled_flag
    push_bits(&mut bits, 1, 1); // output_flag_present_flag
    push_bits(&mut bits, 5, 3); // num_extra_slice_header_bits
    push_bits(&mut bits, 1, 1); // sign_data_hiding_enabled_flag
    push_bits(&mut bits, 1, 1); // cabac_init_present_flag
    push_ue(&mut bits, 1); // num_ref_idx_l0_default_active_minus1
    push_ue(&mut bits, 2); // num_ref_idx_l1_default_active_minus1
    push_se(&mut bits, -2); // init_qp_minus26
    push_bits(&mut bits, 1, 1); // constrained_intra_pred_flag
    push_bits(&mut bits, 1, 1); // transform_skip_enabled_flag
    push_bits(&mut bits, 1, 1); // cu_qp_delta_enabled_flag
    push_ue(&mut bits, 1); // diff_cu_qp_delta_depth
    push_se(&mut bits, -1); // pps_cb_qp_offset
    push_se(&mut bits, 2); // pps_cr_qp_offset
    push_bits(&mut bits, 1, 1); // pps_slice_chroma_qp_offsets_present_flag
    push_bits(&mut bits, 1, 1); // weighted_pred_flag
    push_bits(&mut bits, 0, 1); // weighted_bipred_flag
    push_bits(&mut bits, 1, 1); // transquant_bypass_enabled_flag
    push_bits(&mut bits, 1, 1); // tiles_enabled_flag
    push_bits(&mut bits, 1, 1); // entropy_coding_sync_enabled_flag
    push_ue(&mut bits, 1); // num_tile_columns_minus1
    push_ue(&mut bits, 1); // num_tile_rows_minus1
    push_bits(&mut bits, 0, 1); // uniform_spacing_flag
    push_ue(&mut bits, 3); // column_width_minus1[0]
    push_ue(&mut bits, 4); // row_height_minus1[0]
    push_bits(&mut bits, 1, 1); // loop_filter_across_tiles_enabled_flag
    push_bits(&mut bits, 1, 1); // pps_loop_filter_across_slices_enabled_flag
    push_bits(&mut bits, 1, 1); // deblocking_filter_control_present_flag
    push_bits(&mut bits, 1, 1); // deblocking_filter_override_enabled_flag
    push_bits(&mut bits, 0, 1); // pps_deblocking_filter_disabled_flag
    push_se(&mut bits, -1); // pps_beta_offset_div2
    push_se(&mut bits, 2); // pps_tc_offset_div2
    push_bits(&mut bits, 0, 1); // pps_scaling_list_data_present_flag
    push_bits(&mut bits, 1, 1); // lists_modification_present_flag
    push_ue(&mut bits, 2); // log2_parallel_merge_level_minus2
    push_bits(&mut bits, 1, 1); // slice_segment_header_extension_present_flag
    push_bits(&mut bits, 1, 1); // pps_extension_present_flag
    push_bits(&mut bits, 1, 1); // pps_range_extension_flag
    push_bits(&mut bits, 0, 1); // pps_multilayer_extension_flag
    push_bits(&mut bits, 0, 1); // pps_3d_extension_flag
    push_bits(&mut bits, 1, 1); // pps_scc_extension_flag
    push_bits(&mut bits, 0, 4); // pps_extension_4bits
    push_ue(&mut bits, 1); // log2_max_transform_skip_block_size_minus2
    push_bits(&mut bits, 1, 1); // cross_component_prediction_enabled_flag
    push_bits(&mut bits, 1, 1); // chroma_qp_offset_list_enabled_flag
    push_ue(&mut bits, 1); // diff_cu_chroma_qp_offset_depth
    push_ue(&mut bits, 1); // chroma_qp_offset_list_len_minus1
    push_se(&mut bits, 0);
    push_se(&mut bits, -1);
    push_se(&mut bits, 2);
    push_se(&mut bits, -2);
    push_ue(&mut bits, 2); // log2_sao_offset_scale_luma
    push_ue(&mut bits, 3); // log2_sao_offset_scale_chroma
    push_bits(&mut bits, 1, 1); // pps_curr_pic_ref_enabled_flag
    push_bits(&mut bits, 1, 1); // residual_adaptive_colour_transform_enabled_flag
    push_bits(&mut bits, 1, 1); // pps_slice_act_qp_offsets_present_flag
    push_se(&mut bits, 0);
    push_se(&mut bits, 0);
    push_se(&mut bits, 0);
    push_bits(&mut bits, 1, 1); // pps_palette_predictor_initializers_present_flag
    push_ue(&mut bits, 2); // pps_num_palette_predictor_initializers
    push_bits(&mut bits, 0, 1); // monochrome_palette_flag
    push_ue(&mut bits, 2); // luma_bit_depth_entry_minus8 -> 10 bits
    push_ue(&mut bits, 1); // chroma_bit_depth_entry_minus8 -> 9 bits
    for value in [1, 2] {
        push_bits(&mut bits, value, 10);
    }
    for value in [3, 4, 5, 6] {
        push_bits(&mut bits, value, 9);
    }
    push_bits(&mut bits, 1, 1); // rbsp_stop_one_bit
    push_bits(&mut bits, 0, 7); // rbsp_alignment_zero_bit

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let pps = PictureParameterSetSyntax::parse(&mut reader).unwrap();
    assert_eq!(pps.pps_pic_parameter_set_id, 3);
    assert_eq!(pps.init_qp_minus26, -2);
    assert_eq!(pps.tiles.as_ref().unwrap().column_width_minus1, vec![3]);
    assert_eq!(
        pps.deblocking_filter_control
            .as_ref()
            .unwrap()
            .pps_tc_offset_div2,
        Some(2)
    );
    let extension = pps.pps_extension.as_ref().unwrap();
    assert_eq!(
        extension
            .range_extension
            .as_ref()
            .unwrap()
            .cb_qp_offset_list,
        vec![0, 2]
    );
    let scc = extension.scc_extension.as_ref().unwrap();
    assert_eq!(scc.palette_predictor_initializers[0], vec![1, 2]);
    assert_eq!(scc.palette_predictor_initializers[2], vec![5, 6]);
    assert!(extension.trailing_bits_parsed);
}

#[test]
fn rbsp_parsers_handle_sei_aud_filler_and_empty_units() {
    let mut sei_bits = Vec::new();
    push_bits(&mut sei_bits, 5, 8); // payloadType
    push_bits(&mut sei_bits, 3, 8); // payloadSize
    push_bits(&mut sei_bits, 1, 8);
    push_bits(&mut sei_bits, 2, 8);
    push_bits(&mut sei_bits, 3, 8);
    push_bits(&mut sei_bits, 255, 8); // payloadType continuation
    push_bits(&mut sei_bits, 5, 8); // payloadType final byte -> 260
    push_bits(&mut sei_bits, 1, 8); // payloadSize
    push_bits(&mut sei_bits, 9, 8);
    push_bits(&mut sei_bits, 1, 1); // rbsp_stop_one_bit
    push_bits(&mut sei_bits, 0, 7); // rbsp_alignment_zero_bit
    let sei_bytes = finish_bits(&sei_bits);
    let mut sei_reader = BitReader::new(&sei_bytes);
    let sei = h265rs::SeiRbsp::parse(&mut sei_reader).unwrap();
    assert_eq!(sei.messages.len(), 2);
    assert_eq!(sei.messages[0].payload, vec![1, 2, 3]);
    assert_eq!(sei.messages[1].payload_type, 260);
    assert_eq!(sei.messages[1].payload, vec![9]);

    let aud_bits = pack_bits("01010000"); // pic_type=2, trailing bits
    let mut aud_reader = BitReader::new(&aud_bits);
    assert_eq!(
        h265rs::parse_access_unit_delimiter_rbsp(&mut aud_reader)
            .unwrap()
            .pic_type,
        2
    );

    let filler_bits = pack_bits("111111111111111110000000");
    let mut filler_reader = BitReader::new(&filler_bits);
    assert_eq!(
        h265rs::parse_filler_data_rbsp(&mut filler_reader)
            .unwrap()
            .filler_byte_count,
        2
    );
    assert!(h265rs::parse_end_of_sequence_rbsp(&mut BitReader::new(&[])).is_ok());
    assert!(h265rs::parse_end_of_bitstream_rbsp(&mut BitReader::new(&[])).is_ok());

    let mut trailing_bits = Vec::new();
    push_bits(&mut trailing_bits, 1, 1); // rbsp_stop_one_bit
    push_bits(&mut trailing_bits, 0, 7); // alignment
    push_bits(&mut trailing_bits, 0, 16); // cabac_zero_word
    let trailing_bytes = finish_bits(&trailing_bits);
    let mut trailing_reader = BitReader::new(&trailing_bytes);
    assert_eq!(
        h265rs::parse_rbsp_slice_segment_trailing_bits(&mut trailing_reader).unwrap(),
        1
    );

    let dispatched = h265rs::parse_nal_unit_syntax_from_bytes(&[0x46, 0x01, 0x50]).unwrap();
    assert!(matches!(
        dispatched,
        h265rs::NalUnitSyntax::AccessUnitDelimiter(value) if value.pic_type == 2
    ));
}

#[test]
fn slice_header_parser_handles_idr_and_dependent_segments() {
    let mut idr_bits = Vec::new();
    push_bits(&mut idr_bits, 1, 1); // first_slice_segment_in_pic_flag
    push_bits(&mut idr_bits, 0, 1); // no_output_of_prior_pics_flag
    push_ue(&mut idr_bits, 2); // slice_pic_parameter_set_id
    push_bits(&mut idr_bits, 1, 1); // reserved slice header bit
    push_ue(&mut idr_bits, 2); // slice_type = I
    push_bits(&mut idr_bits, 1, 1); // pic_output_flag
    push_bits(&mut idr_bits, 0, 2); // colour_plane_id
    push_se(&mut idr_bits, 0); // slice_qp_delta
    push_bits(&mut idr_bits, 1, 1); // byte_alignment stop bit
    push_bits(&mut idr_bits, 0, 3); // byte_alignment zero bits
    let idr_bytes = finish_bits(&idr_bits);
    let mut idr_reader = BitReader::new(&idr_bytes);
    let mut idr_context = h265rs::SliceSegmentHeaderContext::new(19, 0, &[]);
    idr_context.output_flag_present_flag = true;
    idr_context.num_extra_slice_header_bits = 1;
    idr_context.separate_colour_plane_flag = true;
    let idr = h265rs::parse_slice_segment_header(&mut idr_reader, &idr_context).unwrap();
    assert_eq!(idr.slice_pic_parameter_set_id, 2);
    assert_eq!(idr.slice_type, Some(2));
    assert_eq!(idr.colour_plane_id, Some(0));
    assert!(idr.reference_picture_set.is_none());

    let mut dependent_bits = Vec::new();
    push_bits(&mut dependent_bits, 0, 1); // first_slice_segment_in_pic_flag
    push_ue(&mut dependent_bits, 2); // slice_pic_parameter_set_id
    push_bits(&mut dependent_bits, 1, 1); // dependent_slice_segment_flag
    push_bits(&mut dependent_bits, 3, 2); // slice_segment_address
    push_bits(&mut dependent_bits, 1, 1); // byte_alignment stop bit
    let dependent_bytes = finish_bits(&dependent_bits);
    let mut dependent_reader = BitReader::new(&dependent_bytes);
    let mut dependent_context = h265rs::SliceSegmentHeaderContext::new(1, 2, &[]);
    dependent_context.dependent_slice_segments_enabled_flag = true;
    let dependent =
        h265rs::parse_slice_segment_header(&mut dependent_reader, &dependent_context).unwrap();
    assert!(dependent.dependent_slice_segment_flag);
    assert_eq!(dependent.slice_segment_address, Some(3));
    assert!(dependent.slice_type.is_none());
}

#[test]
fn cabac_slice_data_primitives_follow_clause_7_conditions() {
    let mut tree = MockCabac::new([1]);
    let tree = h265rs::parse_coding_quadtree_shape(
        &mut tree,
        0,
        0,
        3,
        0,
        h265rs::CodingQuadtreeGeometry {
            pic_width_in_luma_samples: 8,
            pic_height_in_luma_samples: 8,
            min_cb_log2_size: 2,
        },
    )
    .unwrap();
    assert!(tree.split_cu_flag);
    assert_eq!(tree.children.len(), 4);
    assert_eq!(tree.children[3].x, 4);
    assert_eq!(tree.children[3].y, 4);

    let mut mvd = MockCabac::new([1, 0, 1, 2, 1]);
    let mvd = parse_motion_vector_difference(&mut mvd).unwrap();
    assert_eq!(mvd.abs_mvd_greater0_flag, [true, false]);
    assert_eq!(mvd.abs_mvd_minus2, [Some(2), None]);
    assert_eq!(mvd.mvd_sign_flag, [Some(true), None]);

    let mut cross = MockCabac::new([2, 1]);
    let cross = parse_cross_component_prediction(&mut cross).unwrap();
    assert_eq!(cross.log2_res_scale_abs_plus1, 2);
    assert_eq!(cross.res_scale_sign_flag, Some(true));

    let mut delta = MockCabac::new([3, 1]);
    let mut delta_state = DeltaQpState::new();
    delta_state.parse(&mut delta, true).unwrap();
    assert_eq!(delta_state.value, -3);
    assert!(delta_state.coded);

    let mut chroma = MockCabac::new([1, 2]);
    let mut chroma_state = ChromaQpOffsetState::new();
    chroma_state.parse(&mut chroma, true, 2).unwrap();
    assert_eq!(chroma_state.index, Some(2));

    let mut sao = MockCabac::new([1, 0, 1, 0, 2, 0, 1, 3, 0, 0, 0, 0, 0]);
    let sao = parse_sao(&mut sao, false, false, true, true, true).unwrap();
    assert_eq!(sao.luma.as_ref().unwrap().type_idx, 1);
    assert_eq!(sao.luma.as_ref().unwrap().offset_sign, vec![false, true]);
    assert_eq!(sao.chroma.as_ref().unwrap().type_idx, 0);

    let mut prediction = MockCabac::new([0, 0, 0, 1]);
    let prediction = h265rs::parse_prediction_unit(
        &mut prediction,
        h265rs::PredictionUnitContext {
            slice_type: 1,
            num_ref_idx_l0_active_minus1: 0,
            num_ref_idx_l1_active_minus1: 0,
            five_minus_max_num_merge_cand: 0,
            mvd_l1_zero_flag: false,
        },
        false,
    )
    .unwrap();
    assert_eq!(prediction.merge_flag, Some(false));
    assert_eq!(
        prediction.mvd_l0.as_ref().unwrap().abs_mvd_greater0_flag,
        [false, false]
    );
    assert_eq!(prediction.mvp_l0_flag, Some(true));
}

#[test]
fn coding_unit_and_transform_tree_follow_recursive_boundaries() {
    let coding_context = h265rs::CodingUnitContext {
        slice_type: 2,
        transquant_bypass_enabled_flag: false,
        cu_qp_delta_enabled_flag: false,
        cu_chroma_qp_offset_enabled_flag: false,
        palette_mode_enabled_flag: false,
        pcm_enabled_flag: false,
        log2_cb_size: 2,
        min_cb_log2_size: 2,
        log2_min_ipcm_cb_size: 3,
        log2_max_ipcm_cb_size: 5,
        max_tb_log2_size: 5,
        chroma_array_type: 1,
        palette_max_size: 0,
        predictor_palette_size: 0,
        chroma_qp_offset_list_len_minus1: 0,
        prediction: h265rs::PredictionUnitContext {
            slice_type: 2,
            num_ref_idx_l0_active_minus1: 0,
            num_ref_idx_l1_active_minus1: 0,
            five_minus_max_num_merge_cand: 0,
            mvd_l1_zero_flag: false,
        },
    };
    // part_mode=0, prev_luma_pred_flag=1, mpm_idx=2,
    // intra_chroma_pred_mode=0; rqt_root_cbf is inferred for intra CUs.
    let mut coding = MockCabac::new([0, 1, 2, 0]);
    let unit = h265rs::parse_coding_unit(&mut coding, coding_context).unwrap();
    assert_eq!(unit.part_mode, 0);
    assert_eq!(
        unit.intra_prediction.as_ref().unwrap().mpm_idx,
        vec![Some(2)]
    );
    assert_eq!(unit.rqt_root_cbf, None);

    let transform_context = h265rs::TransformTreeContext {
        cu_pred_mode_intra: true,
        chroma_array_type: 0,
        min_tb_log2_size: 2,
        max_tb_log2_size: 5,
        max_trafo_depth: 3,
        intra_split_flag: false,
        residual_adaptive_colour_transform_enabled_flag: false,
        cross_component_prediction_enabled_flag: false,
        transform_skip_enabled_flag: false,
        log2_max_transform_skip_size: 2,
        explicit_rdpcm_enabled_flag: false,
        implicit_rdpcm_enabled_flag: false,
        intra_luma_pred_mode: 0,
        sign_data_hiding_enabled_flag: false,
        cu_transquant_bypass_flag: false,
        scan_idx: 0,
    };
    // At the minimum transform size split is inferred false; cbf_luma=0.
    let mut transform = MockCabac::new([0]);
    let tree = h265rs::parse_transform_tree(&mut transform, transform_context, 0, 0, 2).unwrap();
    assert!(!tree.split_transform_flag);
    assert_eq!(tree.cbf_luma, Some(false));
    assert!(tree.transform_unit.is_none());
}

#[test]
fn slice_segment_data_parses_ctu_and_end_flag() {
    let context = h265rs::SliceSegmentDataContext {
        start_ctb_addr_in_ts: 0,
        pic_width_in_ctbs: 1,
        slice_addr_rs: 0,
        tiles_enabled_flag: false,
        entropy_coding_sync_enabled_flag: false,
        tile_ids: &[0],
        ctb_addr_in_ts_to_rs: &[0],
        ctb_addr_rs_to_ts: &[0],
        slice_sao_luma_flag: false,
        slice_sao_chroma_flag: false,
        chroma_array_type_nonzero: false,
        geometry: h265rs::CodingQuadtreeGeometry {
            pic_width_in_luma_samples: 4,
            pic_height_in_luma_samples: 4,
            min_cb_log2_size: 2,
        },
        coding_unit: h265rs::CodingUnitContext {
            slice_type: 2,
            transquant_bypass_enabled_flag: false,
            cu_qp_delta_enabled_flag: false,
            cu_chroma_qp_offset_enabled_flag: false,
            palette_mode_enabled_flag: false,
            pcm_enabled_flag: false,
            log2_cb_size: 2,
            min_cb_log2_size: 2,
            log2_min_ipcm_cb_size: 3,
            log2_max_ipcm_cb_size: 5,
            max_tb_log2_size: 5,
            chroma_array_type: 0,
            palette_max_size: 0,
            predictor_palette_size: 0,
            chroma_qp_offset_list_len_minus1: 0,
            prediction: h265rs::PredictionUnitContext {
                slice_type: 2,
                num_ref_idx_l0_active_minus1: 0,
                num_ref_idx_l1_active_minus1: 0,
                five_minus_max_num_merge_cand: 0,
                mvd_l1_zero_flag: false,
            },
        },
    };
    let mut cabac = MockCabac::new([0, 1, 2, 1]);
    let data = h265rs::parse_slice_segment_data(&mut cabac, context).unwrap();
    assert_eq!(data.coding_tree_units.len(), 1);
    assert!(data.coding_tree_units[0].end_of_slice_segment_flag);
    assert_eq!(data.coding_tree_units[0].ctb_addr_in_rs, 0);
}

#[test]
fn palette_coding_parses_predictor_and_scan_runs() {
    let mut cabac = MockCabac::new([0]);
    let palette = h265rs::parse_palette_coding(
        &mut cabac,
        h265rs::PaletteCodingContext {
            n_cb_s: 4,
            predictor_palette_size: 0,
            palette_max_size: 4,
            chroma_array_type: 0,
            cu_qp_delta_enabled_flag: false,
            cu_chroma_qp_offset_enabled_flag: false,
            chroma_qp_offset_list_len_minus1: 0,
            cu_transquant_bypass_flag: false,
        },
    )
    .unwrap();
    assert_eq!(palette.num_signalled_palette_entries, 0);
    assert!(!palette.palette_escape_val_present_flag);
    assert_eq!(palette.runs[0].run_length, 16);
}

#[test]
fn extension_selectors_parse_multilayer_and_3d_bodies() {
    let mut sps_bits = Vec::new();
    push_bits(&mut sps_bits, 0b0110, 4); // range=0, multilayer=1, 3D=1, SCC=0
    push_bits(&mut sps_bits, 0, 4); // sps_extension_4bits
    push_bits(&mut sps_bits, 1, 1); // inter_view_mv_vert_constraint_flag
    push_bits(&mut sps_bits, 0, 4); // iv_di_mc and iv_mv_scal flags
    push_ue(&mut sps_bits, 0); // log2_ivmc_sub_pb_size_minus3
    push_bits(&mut sps_bits, 0, 4); // texture-view flags
    push_bits(&mut sps_bits, 0, 1); // tex_mc_enabled_flag
    push_ue(&mut sps_bits, 0); // log2_texmc_sub_pb_size_minus3
    push_bits(&mut sps_bits, 0, 5); // depth-view flags
    sps_bits.push(true); // rbsp_stop_one_bit
    let sps_bytes = finish_bits(&sps_bits);
    let mut sps_reader = BitReader::new(&sps_bytes);
    let sps = h265rs::SpsExtensionSyntax::parse(&mut sps_reader, 1, 0, 0).unwrap();
    assert!(
        sps.multilayer_extension
            .unwrap()
            .inter_view_mv_vert_constraint_flag
    );
    assert!(sps.three_d_extension.is_some());
    assert!(sps.trailing_bits_parsed);

    let mut pps_bits = Vec::new();
    push_bits(&mut pps_bits, 0b0110, 4); // range=0, multilayer=1, 3D=1, SCC=0
    push_bits(&mut pps_bits, 0, 4); // pps_extension_4bits
    push_bits(&mut pps_bits, 0, 2); // poc reset and infer scaling list
    push_ue(&mut pps_bits, 0); // no reference-location offsets
    push_bits(&mut pps_bits, 0, 1); // colour mapping disabled
    push_bits(&mut pps_bits, 0, 1); // dlts_present_flag
    pps_bits.push(true); // rbsp_stop_one_bit
    let pps_bytes = finish_bits(&pps_bits);
    let mut pps_reader = BitReader::new(&pps_bytes);
    let pps = h265rs::PpsExtensionSyntax::parse(&mut pps_reader, false).unwrap();
    assert!(pps
        .multilayer_extension
        .unwrap()
        .reference_location_offsets
        .is_empty());
    assert!(!pps.three_d_extension.unwrap().dlts_present_flag);
    assert!(pps.trailing_bits_parsed);
}

#[test]
fn slice_segment_layer_parser_composes_header_and_trailing_bits() {
    let header_bytes = pack_bits("01111100");
    let mut reader = BitReader::new(&header_bytes);
    let mut context = h265rs::SliceSegmentHeaderContext::new(1, 2, &[]);
    context.dependent_slice_segments_enabled_flag = true;
    let mut cabac = MockCabac::new([]);
    let layer =
        h265rs::parse_slice_segment_layer_rbsp(&mut reader, &mut cabac, &context, None).unwrap();
    assert!(layer.header.dependent_slice_segment_flag);
    assert!(layer.data.is_none());
    assert_eq!(layer.cabac_zero_word_count, 0);
}

#[test]
fn residual_coding_infers_the_last_dc_coefficient() {
    let mut cabac = MockCabac::new([0, 0, 0, 0]);
    let residual = h265rs::parse_residual_coding(
        &mut cabac,
        h265rs::ResidualCodingContext {
            log2_trafo_size: 2,
            cu_pred_mode_intra: true,
            transform_skip_enabled_flag: false,
            log2_max_transform_skip_size: 2,
            explicit_rdpcm_enabled_flag: false,
            implicit_rdpcm_enabled_flag: false,
            intra_luma_pred_mode: 0,
            sign_data_hiding_enabled_flag: false,
            cu_transquant_bypass_flag: false,
            scan_idx: 0,
        },
    )
    .unwrap();
    assert_eq!(
        residual
            .sig_coeff_flags
            .iter()
            .filter(|flag| **flag)
            .count(),
        1
    );
    assert_eq!(residual.coefficients, vec![1]);
}
