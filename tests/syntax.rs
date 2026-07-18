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
