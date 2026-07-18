#![allow(missing_docs)]

use h265rs::{
    decode_bins, derive_cabac_init_type, encode_bins, map_signed_code_num, map_signed_value,
    Binarization, CabacContext, CabacContextTable, CabacDecoder, CabacReader,
};

fn pack_bits(bits: &[bool]) -> Vec<u8> {
    let mut bytes = vec![0_u8; bits.len().div_ceil(8)];
    for (index, &bit) in bits.iter().enumerate() {
        if bit {
            bytes[index / 8] |= 1 << (7 - index % 8);
        }
    }
    bytes
}

#[test]
fn generic_clause9_binarizations_round_trip() {
    for value in 0..=31 {
        for binarization in [
            Binarization::Fixed { c_max: 31 },
            Binarization::TruncatedBinary { c_max: 31 },
            Binarization::TruncatedRice {
                c_max: 31,
                rice_parameter: 2,
            },
            Binarization::ExpGolomb { order: 0 },
            Binarization::ExpGolomb { order: 2 },
            Binarization::LimitedExpGolomb {
                rice_parameter: 2,
                log2_transform_range: 15,
            },
        ] {
            let bins = encode_bins(value, binarization).unwrap();
            assert_eq!(
                decode_bins(&bins, binarization),
                Ok(value),
                "value={value}, binarization={binarization:?}, bins={bins:?}"
            );
        }
    }
}

#[test]
fn limited_exp_golomb_rejects_an_inconsistent_prefix_suffix() {
    assert!(h265rs::decode_bins(
        &[false, true],
        Binarization::LimitedExpGolomb {
            rice_parameter: 0,
            log2_transform_range: 15,
        }
    )
    .is_err());
}

#[test]
fn signed_exp_golomb_mapping_matches_table_9_3() {
    let expected = [0, 1, -1, 2, -2, 3, -3];
    for (code_num, &value) in expected.iter().enumerate() {
        assert_eq!(map_signed_code_num(code_num as u64), value);
        assert_eq!(map_signed_value(value), code_num as u64);
    }
}

#[test]
fn cabac_context_initialization_uses_init_value_and_slice_qp() {
    assert_eq!(
        CabacContext::from_init_value(0x00, 22),
        CabacContext::new(62, 0)
    );
    assert_eq!(
        CabacContext::from_init_value(0xff, 22),
        CabacContext::new(62, 1)
    );
}

#[test]
fn cabac_decodes_mps_bypass_and_termination_bins() {
    let mut decoder = CabacDecoder::new(&[0; 8], 1).unwrap();
    assert_eq!(decoder.decode_decision(0), Ok(0));
    assert_eq!(decoder.decode_bypass(), Ok(0));
    assert_eq!(decoder.decode_terminate(), Ok(0));
    assert!(!decoder.is_terminated());
}

#[test]
fn cabac_decodes_an_lps_from_the_initial_interval() {
    // 382 is the first offset that selects the initial pStateIdx=0 LPS range.
    let data = [0b1011_1111, 0b0000_0000, 0, 0];
    let mut decoder = CabacDecoder::new(&data, 1).unwrap();
    assert_eq!(decoder.decode_decision(0), Ok(1));
}

#[test]
fn standard_context_tables_use_clause_9_init_offsets() {
    for init_type in 0..=2 {
        CabacDecoder::with_standard_contexts(&[0; 128], init_type, 22).unwrap();
    }
    let decoder = CabacDecoder::with_standard_contexts(&[0; 128], 1, 22).unwrap();
    let split = CabacContextTable::SplitCu.context_index(3).unwrap();
    assert_eq!(
        decoder.contexts()[split],
        CabacContext::from_init_value(107, 22)
    );

    let part_mode = CabacContextTable::PartMode.context_index(1).unwrap();
    assert_eq!(
        decoder.contexts()[part_mode],
        CabacContext::from_init_value(154, 22)
    );

    let sig_coeff = CabacContextTable::SigCoeff.context_index(42).unwrap();
    assert_eq!(
        decoder.contexts()[sig_coeff],
        CabacContext::from_init_value(155, 22)
    );

    let decoder = CabacDecoder::with_standard_contexts(&[0; 128], 1, 22).unwrap();
    let luma_transform_skip = CabacContextTable::TransformSkip.context_index(1).unwrap();
    let chroma_transform_skip = CabacContextTable::TransformSkip.context_index(4).unwrap();
    assert_eq!(
        decoder.contexts()[luma_transform_skip],
        CabacContext::from_init_value(139, 22)
    );
    assert_eq!(
        decoder.contexts()[chroma_transform_skip],
        CabacContext::from_init_value(139, 22)
    );
}

#[test]
fn sig_coeff_initialization_groups_are_42_contexts_apart() {
    for init_type in 0..=2 {
        let decoder = CabacDecoder::with_standard_contexts(&[0; 128], init_type, 22).unwrap();
        let normal = CabacContextTable::SigCoeff
            .context_index(init_type as usize * 42)
            .unwrap();
        assert_eq!(
            normal,
            CabacContextTable::SigCoeff.context_index(0).unwrap() + init_type as usize * 42
        );
        let special = CabacContextTable::SigCoeff
            .context_index(126 + init_type as usize * 2)
            .unwrap();
        assert_eq!(
            decoder.contexts()[special],
            CabacContext::from_init_value(if init_type == 0 { 141 } else { 140 }, 22)
        );
        assert_eq!(
            decoder
                .syntax_context_index(CabacContextTable::SigCoeff, 42)
                .unwrap(),
            special
        );
    }
}

#[test]
fn cabac_contexts_can_be_reset_for_a_new_tile_substream() {
    let mut decoder = CabacDecoder::with_standard_contexts(&[0; 128], 1, 22).unwrap();
    let index = CabacContextTable::SplitCu.context_index(0).unwrap();
    let initial = decoder.contexts()[index];
    decoder.context_mut(index).unwrap().state_index = 0;
    decoder.context_mut(index).unwrap().value_mps = 1;
    decoder.reset_contexts_to_initial();
    assert_eq!(decoder.contexts()[index], initial);
}

#[test]
fn cabac_wpp_contexts_round_trip_adapted_state() {
    let mut decoder = CabacDecoder::with_standard_contexts(&[0; 128], 1, 22).unwrap();
    let index = CabacContextTable::SplitCu.context_index(0).unwrap();
    decoder.context_mut(index).unwrap().state_index = 7;
    decoder.store_wpp_contexts();
    decoder.context_mut(index).unwrap().state_index = 19;
    assert!(decoder.synchronize_wpp_contexts());
    assert_eq!(decoder.contexts()[index].state_index, 7);
}

#[test]
fn syntax_context_increments_apply_init_type_offsets() {
    let decoder = CabacDecoder::with_standard_contexts(&[0; 128], 2, 22).unwrap();
    let flat = decoder
        .syntax_context_index(CabacContextTable::AbsMvd, 0)
        .unwrap();
    let expected = CabacContextTable::AbsMvd.context_index(2).unwrap();
    assert_eq!(flat, expected);
    assert_eq!(
        decoder.contexts()[flat],
        CabacContext::from_init_value(169, 22)
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::TransformSkip, 3)
            .unwrap(),
        CabacContextTable::TransformSkip.context_index(5).unwrap()
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::SigCoeff, 42)
            .unwrap(),
        CabacContextTable::SigCoeff.context_index(130).unwrap()
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::CbfChroma, 12)
            .unwrap(),
        CabacContextTable::CbfChroma.context_index(12).unwrap()
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::AbsMvd, 0)
            .unwrap(),
        CabacContextTable::AbsMvd.context_index(2).unwrap()
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::AbsMvd, 1)
            .unwrap(),
        CabacContextTable::AbsMvd.context_index(3).unwrap()
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::ExplicitRdpcm, 0)
            .unwrap(),
        CabacContextTable::ExplicitRdpcm.context_index(1).unwrap()
    );
    assert_eq!(
        decoder
            .syntax_context_index(CabacContextTable::ExplicitRdpcm, 2)
            .unwrap(),
        CabacContextTable::ExplicitRdpcm.context_index(3).unwrap()
    );
}

#[test]
fn cabac_rejects_reserved_initial_offsets() {
    assert!(CabacDecoder::new(&[0xff, 0x80], 1).is_err());
    assert!(CabacDecoder::new(&[0xff, 0xc0], 1).is_err());
}

#[test]
fn cabac_pcm_path_reinitializes_the_arithmetic_engine() {
    let mut bits = vec![false; 9];
    bits.extend(std::iter::repeat_n(false, 7));
    for bit in (0..8).rev() {
        bits.push((0x5a_u8 >> bit) & 1 != 0);
    }
    bits.extend(std::iter::repeat_n(false, 8));
    bits.push(true);
    let data = pack_bits(&bits);
    let mut decoder = CabacDecoder::new(&data, 1).unwrap();
    let (luma, chroma) = decoder.read_pcm_sample_values(1, 0, 8, 8).unwrap();
    assert_eq!(luma, vec![0x5a]);
    assert!(chroma.is_empty());
    assert_eq!(decoder.interval_range, 510);
    assert_eq!(decoder.offset, 1);
}

#[test]
fn cabac_byte_alignment_validates_one_then_zero_bits() {
    let mut bits = vec![false; 9];
    bits.push(true);
    bits.extend(std::iter::repeat_n(false, 6));
    let data = pack_bits(&bits);
    let mut decoder = CabacDecoder::new(&data, 1).unwrap();
    CabacReader::byte_alignment(&mut decoder).unwrap();
    assert_eq!(decoder.bit_position(), 16);

    let mut invalid_bits = vec![false; 16];
    invalid_bits[9] = true;
    invalid_bits[10] = true;
    let invalid_data = pack_bits(&invalid_bits);
    let mut decoder = CabacDecoder::new(&invalid_data, 1).unwrap();
    assert!(CabacReader::byte_alignment(&mut decoder).is_err());
}

#[test]
fn cabac_rbsp_trailing_bits_use_the_termination_bit_as_stop_bit() {
    // ivlOffset = 509 (111111101), so DecodeTerminate returns one and the
    // final initialization bit is the rbsp_stop_one_bit.
    let mut decoder = CabacDecoder::new(&[0xfe, 0x80], 1).unwrap();
    assert_eq!(decoder.rbsp_slice_segment_trailing_bits(), Ok(0));
    assert_eq!(decoder.bit_position(), 16);
}

#[test]
fn cabac_subset_alignment_consumes_zero_bits_after_termination() {
    let mut decoder = CabacDecoder::new(&[0xfe, 0x80], 1).unwrap();
    assert_eq!(decoder.decode_terminate(), Ok(1));
    CabacReader::byte_alignment(&mut decoder).unwrap();
    assert_eq!(decoder.bit_position(), 16);
}

#[test]
fn init_type_follows_slice_type_and_cabac_flag() {
    assert_eq!(derive_cabac_init_type(2, false), Ok(0));
    assert_eq!(derive_cabac_init_type(1, false), Ok(1));
    assert_eq!(derive_cabac_init_type(1, true), Ok(2));
    assert_eq!(derive_cabac_init_type(0, false), Ok(2));
    assert_eq!(derive_cabac_init_type(0, true), Ok(1));
    assert!(derive_cabac_init_type(3, false).is_err());
}
