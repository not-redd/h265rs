use super::{
    parse_access_unit_delimiter_rbsp, parse_end_of_bitstream_rbsp, parse_end_of_sequence_rbsp,
    parse_filler_data_rbsp, AccessUnitDelimiterRbsp, BitReader, FillerDataRbsp, NalUnitHeader,
    ParsedNalUnit, PictureParameterSetSyntax, SeiRbsp, SequenceParameterSetSyntax, SyntaxError,
    VideoParameterSetSyntax,
};

/// Parsed RBSP syntax selected by the NAL unit type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NalUnitSyntax {
    /// Video parameter set RBSP, NAL unit type 32.
    VideoParameterSet(Box<VideoParameterSetSyntax>),
    /// Sequence parameter set RBSP, NAL unit type 33.
    SequenceParameterSet(Box<SequenceParameterSetSyntax>),
    /// Picture parameter set RBSP, NAL unit type 34.
    PictureParameterSet(Box<PictureParameterSetSyntax>),
    /// Supplemental enhancement information RBSP, NAL unit types 39 and 40.
    SupplementalEnhancementInformation(Box<SeiRbsp>),
    /// Access-unit delimiter RBSP, NAL unit type 35.
    AccessUnitDelimiter(AccessUnitDelimiterRbsp),
    /// End-of-sequence RBSP, NAL unit type 36.
    EndOfSequence,
    /// End-of-bitstream RBSP, NAL unit type 37.
    EndOfBitstream,
    /// Filler-data RBSP, NAL unit type 38.
    FillerData(FillerDataRbsp),
    /// A NAL type whose syntax needs VCL or extension context not carried by
    /// the generic dispatcher.
    Raw(Vec<u8>),
}

/// Parses the complete non-VCL RBSP syntax selected by a parsed NAL header.
pub fn parse_nal_unit_syntax(
    header: &NalUnitHeader,
    rbsp: &[u8],
) -> Result<NalUnitSyntax, SyntaxError> {
    let mut reader = BitReader::new(rbsp);
    let syntax = match header.nal_unit_type {
        32 => {
            NalUnitSyntax::VideoParameterSet(Box::new(VideoParameterSetSyntax::parse(&mut reader)?))
        }
        33 => NalUnitSyntax::SequenceParameterSet(Box::new(SequenceParameterSetSyntax::parse(
            &mut reader,
        )?)),
        34 => NalUnitSyntax::PictureParameterSet(Box::new(PictureParameterSetSyntax::parse(
            &mut reader,
        )?)),
        35 => NalUnitSyntax::AccessUnitDelimiter(parse_access_unit_delimiter_rbsp(&mut reader)?),
        36 => {
            parse_end_of_sequence_rbsp(&mut reader)?;
            NalUnitSyntax::EndOfSequence
        }
        37 => {
            parse_end_of_bitstream_rbsp(&mut reader)?;
            NalUnitSyntax::EndOfBitstream
        }
        38 => NalUnitSyntax::FillerData(parse_filler_data_rbsp(&mut reader)?),
        39 | 40 => NalUnitSyntax::SupplementalEnhancementInformation(Box::new(SeiRbsp::parse(
            &mut reader,
        )?)),
        _ => return Ok(NalUnitSyntax::Raw(rbsp.to_vec())),
    };
    if reader.bits_remaining() != 0 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "RBSP parser did not consume the complete payload",
        ));
    }
    Ok(syntax)
}

/// Parses a complete NAL unit and dispatches its RBSP syntax.
pub fn parse_nal_unit_syntax_from_bytes(nal_unit: &[u8]) -> Result<NalUnitSyntax, SyntaxError> {
    let parsed = ParsedNalUnit::parse(nal_unit)?;
    parse_nal_unit_syntax(&parsed.header, &parsed.rbsp)
}
