#![allow(missing_docs)]

use h265rs::{
    ebsp_to_rbsp, BitReader, NalUnitHeader, ParsedNalUnit, ScalingListData,
    SequenceParameterSetHeader, SequenceParameterSetSyntax, SyntaxDescriptor, SyntaxError,
    SyntaxValue, VideoParameterSetHeader,
};

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

    let bytes = finish_bits(&bits);
    let mut reader = BitReader::new(&bytes);
    let vps = VideoParameterSetHeader::parse(&mut reader).unwrap();
    assert_eq!(vps.vps_video_parameter_set_id, 3);
    assert_eq!(vps.vps_reserved_0xffff_16bits, 0xffff);
    assert_eq!(vps.profile_tier_level.general_level_idc, 120);
    assert_eq!(vps.sub_layer_ordering_info.len(), 1);
    assert!(!vps.vps_timing_info_present_flag);
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
