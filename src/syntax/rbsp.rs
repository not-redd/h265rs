use super::{BitReader, SyntaxError};

/// One generic SEI message from §7.3.5.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeiMessage {
    /// Accumulated `payloadType` value.
    pub payload_type: u64,
    /// Accumulated `payloadSize` value.
    pub payload_size: usize,
    /// Raw payload bytes. Payload-specific parsing belongs to Annex D.
    pub payload: Vec<u8>,
}

/// Parsed SEI RBSP from §7.3.2.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeiRbsp {
    /// Messages in bitstream order.
    pub messages: Vec<SeiMessage>,
}

fn read_ff_coded_value(reader: &mut BitReader<'_>) -> Result<u64, SyntaxError> {
    let mut value = 0u64;
    loop {
        let byte = reader.read_u(8)?;
        value = value
            .checked_add(byte)
            .ok_or(SyntaxError::InvalidSyntaxValue("SEI value overflows u64"))?;
        if byte != 0xff {
            return Ok(value);
        }
    }
}

impl SeiMessage {
    /// Parses one `sei_message()` header and its raw payload.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let payload_type = read_ff_coded_value(reader)?;
        let payload_size_value = read_ff_coded_value(reader)?;
        let payload_size = usize::try_from(payload_size_value)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("SEI payload is too large"))?;
        let mut payload = Vec::with_capacity(payload_size);
        for _ in 0..payload_size {
            payload.push(reader.read_u(8)? as u8);
        }
        Ok(Self {
            payload_type,
            payload_size,
            payload,
        })
    }
}

impl SeiRbsp {
    /// Parses `sei_rbsp()` and consumes `rbsp_trailing_bits()`.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let mut messages = Vec::new();
        loop {
            messages.push(SeiMessage::parse(reader)?);
            if !reader.more_rbsp_data() {
                break;
            }
        }
        reader.read_rbsp_trailing_bits()?;
        Ok(Self { messages })
    }
}

/// Parsed access-unit delimiter RBSP from §7.3.2.5.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessUnitDelimiterRbsp {
    /// `pic_type`.
    pub pic_type: u8,
}

/// Parses `access_unit_delimiter_rbsp()`.
pub fn parse_access_unit_delimiter_rbsp(
    reader: &mut BitReader<'_>,
) -> Result<AccessUnitDelimiterRbsp, SyntaxError> {
    let pic_type = reader.read_u(3)? as u8;
    reader.read_rbsp_trailing_bits()?;
    Ok(AccessUnitDelimiterRbsp { pic_type })
}

/// Parses the empty `end_of_seq_rbsp()` syntax structure.
pub fn parse_end_of_sequence_rbsp(reader: &mut BitReader<'_>) -> Result<(), SyntaxError> {
    parse_empty_rbsp(reader, "end-of-sequence RBSP")
}

/// Parses the empty `end_of_bitstream_rbsp()` syntax structure.
pub fn parse_end_of_bitstream_rbsp(reader: &mut BitReader<'_>) -> Result<(), SyntaxError> {
    parse_empty_rbsp(reader, "end-of-bitstream RBSP")
}

fn parse_empty_rbsp(reader: &BitReader<'_>, name: &'static str) -> Result<(), SyntaxError> {
    if reader.bits_remaining() == 0 {
        Ok(())
    } else {
        Err(SyntaxError::InvalidSyntaxValue(name))
    }
}

/// Parsed filler-data RBSP from §7.3.2.8.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillerDataRbsp {
    /// Number of `ff_byte` values consumed.
    pub filler_byte_count: usize,
}

/// Parses `filler_data_rbsp()` and consumes its trailing bits.
pub fn parse_filler_data_rbsp(reader: &mut BitReader<'_>) -> Result<FillerDataRbsp, SyntaxError> {
    let mut filler_byte_count = 0usize;
    while reader.bits_remaining() >= 8 && reader.next_bits(8)? == 0xff {
        reader.read_u(8)?;
        filler_byte_count = filler_byte_count
            .checked_add(1)
            .ok_or(SyntaxError::InvalidSyntaxValue("too many filler bytes"))?;
    }
    reader.read_rbsp_trailing_bits()?;
    Ok(FillerDataRbsp { filler_byte_count })
}

/// Consumes `rbsp_slice_segment_trailing_bits()` from §7.3.2.10.
///
/// Returns the number of 16-bit `cabac_zero_word` values consumed after the
/// ordinary RBSP trailing bits.
pub fn parse_rbsp_slice_segment_trailing_bits(
    reader: &mut BitReader<'_>,
) -> Result<usize, SyntaxError> {
    reader.read_rbsp_trailing_bits()?;
    let mut cabac_zero_word_count = 0usize;
    while reader.bits_remaining() >= 16 {
        if reader.next_bits(16)? != 0 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "cabac_zero_word must equal 0x0000",
            ));
        }
        reader.read_u(16)?;
        cabac_zero_word_count =
            cabac_zero_word_count
                .checked_add(1)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "too many cabac_zero_word values",
                ))?;
    }
    if reader.bits_remaining() != 0 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "trailing CABAC data must be a whole number of words",
        ));
    }
    Ok(cabac_zero_word_count)
}
