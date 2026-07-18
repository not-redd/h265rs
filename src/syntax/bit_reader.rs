use super::SyntaxError;

/// Syntax descriptors listed in H.265 §7.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxDescriptor {
    /// `ae(v)`, context-adaptive arithmetic coded syntax.
    Arithmetic,
    /// `b(8)`, an arbitrary byte at a byte-aligned position.
    Byte,
    /// `f(n)`, fixed-pattern bit string.
    Fixed(usize),
    /// `i(n)`, signed two's-complement integer.
    Signed(usize),
    /// `se(v)`, signed Exp-Golomb integer.
    SignedExpGolomb,
    /// `st(v)`, null-terminated UTF-8 string.
    String,
    /// `u(n)`, unsigned integer.
    Unsigned(usize),
    /// `ue(v)`, unsigned Exp-Golomb integer.
    UnsignedExpGolomb,
}

/// A value returned by a syntax descriptor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyntaxValue {
    /// Fixed or unsigned bit-string/integer value.
    Unsigned(u64),
    /// Signed integer value.
    Signed(i64),
    /// Null-terminated UTF-8 string.
    String(String),
}

/// MSB-first bit reader implementing the basic functions in H.265 §7.2.
#[derive(Clone, Debug)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_position: usize,
    bit_limit: usize,
}

impl<'a> BitReader<'a> {
    /// Creates a reader positioned at the first bit of `data`.
    pub const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            bit_position: 0,
            bit_limit: data.len() * 8,
        }
    }

    /// Returns the current bit offset from the beginning of the input.
    pub const fn position(&self) -> usize {
        self.bit_position
    }

    /// Returns the total input size in bits.
    pub const fn bit_length(&self) -> usize {
        self.data.len() * 8
    }

    /// Returns the number of unread bits.
    pub const fn bits_remaining(&self) -> usize {
        self.bit_limit.saturating_sub(self.bit_position)
    }

    pub(crate) fn set_substream(
        &mut self,
        start_bit: usize,
        end_bit: usize,
    ) -> Result<(), SyntaxError> {
        if start_bit > end_bit || end_bit > self.data.len() * 8 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "bit-reader substream is outside its input",
            ));
        }
        self.bit_position = start_bit;
        self.bit_limit = end_bit;
        Ok(())
    }

    /// Implements `byte_aligned()`.
    pub const fn byte_aligned(&self) -> bool {
        self.bit_position.is_multiple_of(8)
    }

    /// Implements `read_bits(n)`, reading MSB first and advancing the cursor.
    pub fn read_bits(&mut self, count: usize) -> Result<u64, SyntaxError> {
        if count > 64 {
            return Err(SyntaxError::InvalidBitCount(count));
        }
        if count > self.bits_remaining() {
            return Err(SyntaxError::UnexpectedEnd {
                requested: count,
                remaining: self.bits_remaining(),
            });
        }
        let value = self.peek_bits(count)?;
        self.bit_position += count;
        Ok(value)
    }

    /// Implements `next_bits(n)` without advancing the cursor.
    pub fn next_bits(&self, count: usize) -> Result<u64, SyntaxError> {
        self.peek_bits(count)
    }

    /// Returns the next bits without advancing the cursor.
    pub fn peek_bits(&self, count: usize) -> Result<u64, SyntaxError> {
        if count > 64 {
            return Err(SyntaxError::InvalidBitCount(count));
        }
        if count > self.bits_remaining() {
            return Err(SyntaxError::UnexpectedEnd {
                requested: count,
                remaining: self.bits_remaining(),
            });
        }
        let mut value = 0u64;
        for offset in 0..count {
            value = (value << 1) | u64::from(self.bit_at(self.bit_position + offset));
        }
        Ok(value)
    }

    /// Returns `next_bits(n)`, or zero when fewer than `n` bits remain in a
    /// byte-stream context as specified by §7.2.
    pub fn next_bits_or_zero(&self, count: usize) -> Result<u64, SyntaxError> {
        if count > 64 {
            return Err(SyntaxError::InvalidBitCount(count));
        }
        if count > self.bits_remaining() {
            Ok(0)
        } else {
            self.peek_bits(count)
        }
    }

    /// Implements `u(n)`.
    pub fn read_u(&mut self, count: usize) -> Result<u64, SyntaxError> {
        self.read_bits(count)
    }

    /// Implements `f(n)`.
    pub fn read_f(&mut self, count: usize) -> Result<u64, SyntaxError> {
        self.read_bits(count)
    }

    /// Implements `b(8)`.
    pub fn read_b(&mut self) -> Result<u8, SyntaxError> {
        if !self.byte_aligned() {
            return Err(SyntaxError::NotByteAligned);
        }
        Ok(self.read_bits(8)? as u8)
    }

    /// Implements `i(n)` as a two's-complement signed integer.
    pub fn read_i(&mut self, count: usize) -> Result<i64, SyntaxError> {
        if count > 64 {
            return Err(SyntaxError::InvalidBitCount(count));
        }
        let value = self.read_bits(count)?;
        if count == 0 || count == 64 {
            return Ok(value as i64);
        }
        let sign_bit = 1u64 << (count - 1);
        if value & sign_bit == 0 {
            Ok(value as i64)
        } else {
            Ok((value | (!0u64 << count)) as i64)
        }
    }

    /// Implements `ue(v)`, unsigned zero-th order Exp-Golomb coding.
    pub fn read_ue(&mut self) -> Result<u64, SyntaxError> {
        let mut leading_zero_bits = 0usize;
        while self.bits_remaining() > 0 && self.peek_bits(1)? == 0 {
            self.bit_position += 1;
            leading_zero_bits += 1;
            if leading_zero_bits >= 64 {
                return Err(SyntaxError::ExpGolombOverflow);
            }
        }
        if self.bits_remaining() == 0 {
            return Err(SyntaxError::UnexpectedEnd {
                requested: 1,
                remaining: 0,
            });
        }
        self.bit_position += 1;
        let suffix = self.read_bits(leading_zero_bits)?;
        let base = (1u64 << leading_zero_bits) - 1;
        Ok(base + suffix)
    }

    /// Implements `se(v)`, signed zero-th order Exp-Golomb coding.
    pub fn read_se(&mut self) -> Result<i64, SyntaxError> {
        let code_num = self.read_ue()?;
        if code_num % 2 == 1 {
            Ok(code_num.div_ceil(2) as i64)
        } else {
            Ok(-((code_num / 2) as i64))
        }
    }

    /// Implements `st(v)`, reading a byte-aligned null-terminated UTF-8 string.
    pub fn read_st(&mut self) -> Result<String, SyntaxError> {
        if !self.byte_aligned() {
            return Err(SyntaxError::NotByteAligned);
        }
        let start = self.bit_position / 8;
        let end = self.data[start..]
            .iter()
            .position(|&byte| byte == 0)
            .map(|offset| start + offset)
            .ok_or(SyntaxError::MissingStringTerminator)?;
        let value = std::str::from_utf8(&self.data[start..end])
            .map_err(|_| SyntaxError::InvalidUtf8)?
            .to_owned();
        self.bit_position = (end + 1) * 8;
        Ok(value)
    }

    /// Implements `more_rbsp_data()` using the trailing-one-and-zero rule.
    pub fn more_rbsp_data(&self) -> bool {
        if self.bits_remaining() == 0 || self.bit_at(self.bit_position) == 0 {
            return self.bits_remaining() != 0;
        }
        (self.bit_position + 1..self.bit_length()).any(|position| self.bit_at(position) != 0)
    }

    /// Implements `more_data_in_payload()` for a payload measured in bits.
    pub const fn more_data_in_payload(&self, payload_bits: usize) -> bool {
        !(self.byte_aligned() && self.bit_position == payload_bits)
    }

    /// Consumes `rbsp_trailing_bits()`.
    pub fn read_rbsp_trailing_bits(&mut self) -> Result<(), SyntaxError> {
        if self.read_bits(1)? != 1 {
            return Err(SyntaxError::InvalidAlignmentBit);
        }
        while !self.byte_aligned() {
            if self.read_bits(1)? != 0 {
                return Err(SyntaxError::InvalidAlignmentZero);
            }
        }
        Ok(())
    }

    /// Consumes `byte_alignment()`.
    pub fn read_byte_alignment(&mut self) -> Result<(), SyntaxError> {
        if self.byte_aligned() {
            return Ok(());
        }
        self.read_rbsp_trailing_bits()
    }

    /// Reads a syntax descriptor from the current position.
    pub fn read_descriptor(
        &mut self,
        descriptor: SyntaxDescriptor,
    ) -> Result<SyntaxValue, SyntaxError> {
        match descriptor {
            SyntaxDescriptor::Arithmetic => Err(SyntaxError::ArithmeticCodingUnsupported),
            SyntaxDescriptor::Byte => Ok(SyntaxValue::Unsigned(u64::from(self.read_b()?))),
            SyntaxDescriptor::Fixed(count) | SyntaxDescriptor::Unsigned(count) => {
                Ok(SyntaxValue::Unsigned(self.read_bits(count)?))
            }
            SyntaxDescriptor::Signed(count) => Ok(SyntaxValue::Signed(self.read_i(count)?)),
            SyntaxDescriptor::SignedExpGolomb => Ok(SyntaxValue::Signed(self.read_se()?)),
            SyntaxDescriptor::String => Ok(SyntaxValue::String(self.read_st()?)),
            SyntaxDescriptor::UnsignedExpGolomb => Ok(SyntaxValue::Unsigned(self.read_ue()?)),
        }
    }

    fn bit_at(&self, position: usize) -> u8 {
        (self.data[position / 8] >> (7 - position % 8)) & 1
    }
}
