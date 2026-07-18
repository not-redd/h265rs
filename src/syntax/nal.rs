use super::{BitReader, SyntaxError};

/// The two-byte H.265 NAL-unit header from §7.3.1.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NalUnitHeader {
    /// The syntax-level forbidden-zero bit.
    pub forbidden_zero_bit: bool,
    /// NAL unit type in the range 0..=63.
    pub nal_unit_type: u8,
    /// Layer identifier in the range 0..=63.
    pub nuh_layer_id: u8,
    /// Temporal identifier plus one; zero is forbidden by semantics.
    pub nuh_temporal_id_plus1: u8,
}

impl NalUnitHeader {
    /// Parses and validates a two-byte NAL-unit header.
    pub fn parse(bytes: &[u8]) -> Result<Self, SyntaxError> {
        if bytes.len() < 2 {
            return Err(SyntaxError::NalUnitTooShort);
        }
        let mut reader = BitReader::new(&bytes[..2]);
        let header = Self {
            forbidden_zero_bit: reader.read_f(1)? != 0,
            nal_unit_type: reader.read_u(6)? as u8,
            nuh_layer_id: reader.read_u(6)? as u8,
            nuh_temporal_id_plus1: reader.read_u(3)? as u8,
        };
        if header.forbidden_zero_bit {
            return Err(SyntaxError::InvalidNalHeader(
                "forbidden_zero_bit is not zero",
            ));
        }
        if header.nuh_temporal_id_plus1 == 0 {
            return Err(SyntaxError::InvalidNalHeader(
                "nuh_temporal_id_plus1 is not greater than zero",
            ));
        }
        Ok(header)
    }
}

/// A parsed NAL unit with its header and RBSP payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedNalUnit {
    /// Parsed two-byte header.
    pub header: NalUnitHeader,
    /// RBSP payload after emulation-prevention bytes are removed.
    pub rbsp: Vec<u8>,
}

impl ParsedNalUnit {
    /// Parses a complete NAL unit, including its two-byte header.
    pub fn parse(nal_unit: &[u8]) -> Result<Self, SyntaxError> {
        let header = NalUnitHeader::parse(nal_unit)?;
        Ok(Self {
            header,
            rbsp: ebsp_to_rbsp(&nal_unit[2..]),
        })
    }
}

/// Removes `emulation_prevention_three_byte` values from an EBSP payload.
pub fn ebsp_to_rbsp(ebsp: &[u8]) -> Vec<u8> {
    let mut rbsp = Vec::with_capacity(ebsp.len());
    let mut zero_count = 0usize;
    for &byte in ebsp {
        if zero_count >= 2 && byte == 0x03 {
            zero_count = 0;
            continue;
        }
        rbsp.push(byte);
        if byte == 0 {
            zero_count += 1;
        } else {
            zero_count = 0;
        }
    }
    rbsp
}
