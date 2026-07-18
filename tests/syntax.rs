#![allow(missing_docs)]

use h265rs::{
    ebsp_to_rbsp, BitReader, NalUnitHeader, ParsedNalUnit, SyntaxDescriptor, SyntaxError,
    SyntaxValue,
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
