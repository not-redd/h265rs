use std::fmt;

/// Errors produced while parsing H.265 syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxError {
    /// The requested number of bits is unavailable.
    UnexpectedEnd {
        /// Number of bits requested.
        requested: usize,
        /// Number of bits remaining.
        remaining: usize,
    },
    /// A bit-count argument cannot be represented by the operation.
    InvalidBitCount(usize),
    /// The Exp-Golomb prefix would overflow the supported integer type.
    ExpGolombOverflow,
    /// The current bit position is not byte aligned.
    NotByteAligned,
    /// A required alignment or trailing stop bit was not equal to one.
    InvalidAlignmentBit,
    /// A required alignment bit was not zero.
    InvalidAlignmentZero,
    /// A null-terminated string had no terminator before the end of the input.
    MissingStringTerminator,
    /// A string descriptor did not contain valid UTF-8.
    InvalidUtf8,
    /// A NAL unit is shorter than its two-byte header.
    NalUnitTooShort,
    /// A NAL header contains a forbidden or otherwise invalid value.
    InvalidNalHeader(&'static str),
    /// A syntax value violates a structural range required by the table.
    InvalidSyntaxValue(&'static str),
    /// Context-adaptive arithmetic coding is implemented in Clause 9, not here.
    ArithmeticCodingUnsupported,
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEnd {
                requested,
                remaining,
            } => write!(
                f,
                "unexpected end of bitstream: requested {requested} bits, {remaining} remain"
            ),
            Self::InvalidBitCount(count) => write!(f, "invalid bit count {count}"),
            Self::ExpGolombOverflow => write!(f, "Exp-Golomb code overflows u64"),
            Self::NotByteAligned => write!(f, "syntax descriptor requires byte alignment"),
            Self::InvalidAlignmentBit => write!(f, "alignment bit must equal one"),
            Self::InvalidAlignmentZero => write!(f, "alignment bits must equal zero"),
            Self::MissingStringTerminator => write!(f, "null-terminated string has no terminator"),
            Self::InvalidUtf8 => write!(f, "string descriptor contains invalid UTF-8"),
            Self::NalUnitTooShort => write!(f, "NAL unit is shorter than its two-byte header"),
            Self::InvalidNalHeader(message) => write!(f, "invalid NAL header: {message}"),
            Self::InvalidSyntaxValue(message) => write!(f, "invalid syntax value: {message}"),
            Self::ArithmeticCodingUnsupported => {
                write!(
                    f,
                    "arithmetic-coded syntax requires the Clause 9 CABAC engine"
                )
            }
        }
    }
}

impl std::error::Error for SyntaxError {}
