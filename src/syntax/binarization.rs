//! Clause 9.3.3 bin-string construction and decoding primitives.

use super::SyntaxError;

/// Generic Clause 9.3.3 binarization families.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Binarization {
    /// Fixed-length binarization with values in `0..=c_max`.
    Fixed {
        /// Maximum representable syntax value.
        c_max: u32,
    },
    /// Truncated binary binarization.
    TruncatedBinary {
        /// Maximum representable syntax value.
        c_max: u32,
    },
    /// Truncated Rice binarization.
    TruncatedRice {
        /// Maximum representable syntax value.
        c_max: u32,
        /// Rice parameter `cRiceParam`.
        rice_parameter: u8,
    },
    /// k-th order Exp-Golomb binarization.
    ExpGolomb {
        /// Exp-Golomb order `k`.
        order: u8,
    },
    /// Limited k-th order Exp-Golomb binarization from §9.3.3.4.
    LimitedExpGolomb {
        /// Rice parameter.
        rice_parameter: u8,
        /// `log2TransformRange`.
        log2_transform_range: u8,
    },
}

/// Builds a bin string for a syntax value.
pub fn encode_bins(value: u32, binarization: Binarization) -> Result<Vec<bool>, SyntaxError> {
    match binarization {
        Binarization::Fixed { c_max } => fixed(value, c_max),
        Binarization::TruncatedBinary { c_max } => truncated_binary(value, c_max),
        Binarization::TruncatedRice {
            c_max,
            rice_parameter,
        } => truncated_rice(value, c_max, rice_parameter),
        Binarization::ExpGolomb { order } => exp_golomb(value, order),
        Binarization::LimitedExpGolomb {
            rice_parameter,
            log2_transform_range,
        } => limited_exp_golomb(value, rice_parameter, log2_transform_range),
    }
}

/// Decodes a complete bin string for a syntax value.
pub fn decode_bins(bins: &[bool], binarization: Binarization) -> Result<u32, SyntaxError> {
    let value = match binarization {
        Binarization::Fixed { c_max } => decode_fixed(bins, c_max)?,
        Binarization::TruncatedBinary { c_max } => decode_truncated_binary(bins, c_max)?,
        Binarization::TruncatedRice {
            c_max,
            rice_parameter,
        } => decode_truncated_rice(bins, c_max, rice_parameter)?,
        Binarization::ExpGolomb { order } => decode_exp_golomb(bins, order)?,
        Binarization::LimitedExpGolomb {
            rice_parameter,
            log2_transform_range,
        } => decode_limited_exp_golomb(bins, rice_parameter, log2_transform_range)?,
    };
    Ok(value)
}

/// Clause 9.2.2 signed Exp-Golomb mapping.
pub const fn map_signed_code_num(code_num: u64) -> i64 {
    if code_num & 1 == 1 {
        code_num.div_ceil(2) as i64
    } else {
        -((code_num / 2) as i64)
    }
}

/// Inverse of the signed mapping process.
pub const fn map_signed_value(value: i64) -> u64 {
    if value > 0 {
        (value as u64) * 2 - 1
    } else {
        (-value as u64) * 2
    }
}

fn fixed(value: u32, c_max: u32) -> Result<Vec<bool>, SyntaxError> {
    if value > c_max {
        return Err(SyntaxError::InvalidSyntaxValue(
            "fixed binarization value exceeds cMax",
        ));
    }
    let width = ceil_log2(c_max);
    Ok((0..width)
        .rev()
        .map(|bit| (value >> bit) & 1 != 0)
        .collect())
}

fn truncated_binary(value: u32, c_max: u32) -> Result<Vec<bool>, SyntaxError> {
    if value > c_max {
        return Err(SyntaxError::InvalidSyntaxValue(
            "truncated binary value exceeds cMax",
        ));
    }
    let n = u64::from(c_max) + 1;
    let k = 63 - n.leading_zeros();
    let u = (1_u64 << (k + 1)) - n;
    if u64::from(value) < u {
        fixed(value, ((1_u64 << k) - 1) as u32)
    } else {
        fixed(
            (u64::from(value) + u) as u32,
            ((1_u64 << (k + 1)) - 1) as u32,
        )
    }
}

fn truncated_rice(value: u32, c_max: u32, rice: u8) -> Result<Vec<bool>, SyntaxError> {
    if value > c_max {
        return Err(SyntaxError::InvalidSyntaxValue(
            "truncated Rice value exceeds cMax",
        ));
    }
    if rice >= 32 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "truncated-Rice parameter is too large",
        ));
    }
    let prefix = value >> rice;
    let mut bins = vec![true; prefix.min(c_max >> rice) as usize];
    if c_max > value {
        bins.push(false);
    }
    if c_max > value && rice != 0 {
        bins.extend((0..rice).rev().map(|bit| (value >> bit) & 1 != 0));
    }
    Ok(bins)
}

fn exp_golomb(mut value: u32, mut order: u8) -> Result<Vec<bool>, SyntaxError> {
    let mut bins = Vec::new();
    loop {
        let threshold = 1_u32
            .checked_shl(u32::from(order))
            .ok_or(SyntaxError::ExpGolombOverflow)?;
        if value >= threshold {
            bins.push(true);
            value -= threshold;
            order = order.checked_add(1).ok_or(SyntaxError::ExpGolombOverflow)?;
        } else {
            bins.push(false);
            for bit in (0..order).rev() {
                bins.push((value >> bit) & 1 != 0);
            }
            return Ok(bins);
        }
    }
}

fn limited_exp_golomb(
    value: u32,
    rice_parameter: u8,
    log2_transform_range: u8,
) -> Result<Vec<bool>, SyntaxError> {
    if rice_parameter >= 32 || !(1..=28).contains(&log2_transform_range) {
        return Err(SyntaxError::InvalidSyntaxValue(
            "limited EGk parameters are out of range",
        ));
    }
    let max_prefix_extension_length = 28 - log2_transform_range;
    let code_value = value >> rice_parameter;
    let mut prefix_extension_length = 0_u8;
    let mut bins = Vec::new();
    while prefix_extension_length < max_prefix_extension_length
        && u64::from(code_value) > ((2_u64 << prefix_extension_length) - 2)
    {
        bins.push(true);
        prefix_extension_length += 1;
    }
    let escape_length = if prefix_extension_length == max_prefix_extension_length {
        log2_transform_range
    } else {
        bins.push(false);
        prefix_extension_length + rice_parameter
    };
    let base = ((1_u64 << prefix_extension_length) - 1) << rice_parameter;
    let symbol_value =
        u64::from(value)
            .checked_sub(base)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "limited EGk value is below its prefix base",
            ))?;
    if escape_length < 32 && symbol_value >= (1_u64 << escape_length) {
        return Err(SyntaxError::InvalidSyntaxValue(
            "limited EGk suffix exceeds its coded width",
        ));
    }
    bins.extend(
        (0..escape_length)
            .rev()
            .map(|bit| (symbol_value >> bit) & 1 != 0),
    );
    Ok(bins)
}

fn decode_fixed(bins: &[bool], c_max: u32) -> Result<u32, SyntaxError> {
    let width = ceil_log2(c_max);
    if bins.len() != width as usize {
        return Err(SyntaxError::InvalidSyntaxValue(
            "fixed bin string has the wrong length",
        ));
    }
    let value = bins
        .iter()
        .fold(0_u32, |value, &bit| (value << 1) | u32::from(bit));
    (value <= c_max)
        .then_some(value)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "fixed bin string exceeds cMax",
        ))
}

fn decode_truncated_binary(bins: &[bool], c_max: u32) -> Result<u32, SyntaxError> {
    let n = u64::from(c_max) + 1;
    let k = 63 - n.leading_zeros();
    let u = (1_u64 << (k + 1)) - n;
    if bins.len() >= k as usize
        && bins.len() == k as usize
        && bins.iter().fold(0_u64, |v, &b| (v << 1) | u64::from(b)) < u
    {
        return decode_fixed(bins, ((1_u64 << k) - 1) as u32);
    }
    if bins.len() != (k + 1) as usize {
        return Err(SyntaxError::InvalidSyntaxValue(
            "truncated binary bin string has the wrong length",
        ));
    }
    let raw = bins.iter().fold(0_u64, |v, &b| (v << 1) | u64::from(b));
    Ok((raw - u) as u32)
}

fn decode_truncated_rice(bins: &[bool], c_max: u32, rice: u8) -> Result<u32, SyntaxError> {
    let zero = bins.iter().position(|&bit| !bit).unwrap_or(bins.len());
    if zero == bins.len() {
        let expected = (c_max >> rice) as usize;
        if bins.len() != expected {
            return Err(SyntaxError::InvalidSyntaxValue(
                "truncated Rice bin string has the wrong length",
            ));
        }
        return Ok(c_max);
    }
    let prefix_value = (zero as u32) << rice;
    let suffix_len = rice as usize;
    if bins.len() != zero + 1 + suffix_len {
        return Err(SyntaxError::InvalidSyntaxValue(
            "truncated Rice bin string has the wrong length",
        ));
    }
    let mut suffix = 0_u32;
    if suffix_len != 0 {
        for &bit in &bins[zero + 1..] {
            suffix = (suffix << 1) | u32::from(bit);
        }
    }
    let value = prefix_value + suffix;
    if value > c_max {
        return Err(SyntaxError::InvalidSyntaxValue(
            "truncated Rice bin string exceeds cMax",
        ));
    }
    Ok(value)
}

fn decode_exp_golomb(bins: &[bool], mut order: u8) -> Result<u32, SyntaxError> {
    let mut index = 0;
    let mut value = 0_u32;
    loop {
        let bit = *bins.get(index).ok_or(SyntaxError::UnexpectedEnd {
            requested: 1,
            remaining: 0,
        })?;
        index += 1;
        if bit {
            let threshold = 1_u32
                .checked_shl(u32::from(order))
                .ok_or(SyntaxError::ExpGolombOverflow)?;
            value = value
                .checked_add(threshold)
                .ok_or(SyntaxError::ExpGolombOverflow)?;
            order = order.checked_add(1).ok_or(SyntaxError::ExpGolombOverflow)?;
        } else {
            let mut suffix = 0_u32;
            for _ in 0..order {
                suffix = (suffix << 1)
                    | u32::from(*bins.get(index).ok_or(SyntaxError::UnexpectedEnd {
                        requested: 1,
                        remaining: 0,
                    })?);
                index += 1;
            }
            if index != bins.len() {
                return Err(SyntaxError::InvalidSyntaxValue(
                    "Exp-Golomb bin string has trailing bins",
                ));
            }
            return value
                .checked_add(suffix)
                .ok_or(SyntaxError::ExpGolombOverflow);
        }
    }
}

fn decode_limited_exp_golomb(
    bins: &[bool],
    rice_parameter: u8,
    log2_transform_range: u8,
) -> Result<u32, SyntaxError> {
    if rice_parameter >= 32 || !(1..=28).contains(&log2_transform_range) {
        return Err(SyntaxError::InvalidSyntaxValue(
            "limited EGk parameters are out of range",
        ));
    }
    let max_prefix_extension_length = 28 - log2_transform_range;
    let mut prefix_extension_length = 0_u8;
    let mut index = 0_usize;
    while prefix_extension_length < max_prefix_extension_length {
        let bit = *bins.get(index).ok_or(SyntaxError::UnexpectedEnd {
            requested: 1,
            remaining: 0,
        })?;
        index += 1;
        if !bit {
            break;
        }
        prefix_extension_length += 1;
    }
    let escape_length = if prefix_extension_length == max_prefix_extension_length {
        log2_transform_range
    } else {
        prefix_extension_length + rice_parameter
    };
    let mut suffix = 0_u64;
    for _ in 0..escape_length {
        suffix = (suffix << 1)
            | u64::from(*bins.get(index).ok_or(SyntaxError::UnexpectedEnd {
                requested: 1,
                remaining: 0,
            })?);
        index += 1;
    }
    if index != bins.len() {
        return Err(SyntaxError::InvalidSyntaxValue(
            "limited EGk bin string has trailing bins",
        ));
    }
    let base = ((1_u64 << prefix_extension_length) - 1) << rice_parameter;
    let value = base
        .checked_add(suffix)
        .ok_or(SyntaxError::ExpGolombOverflow)?;
    if prefix_extension_length < max_prefix_extension_length
        && (value >> rice_parameter) > ((2_u64 << prefix_extension_length) - 2)
    {
        return Err(SyntaxError::InvalidSyntaxValue(
            "limited EGk prefix and suffix are inconsistent",
        ));
    }
    u32::try_from(value).map_err(|_| SyntaxError::ExpGolombOverflow)
}

const fn ceil_log2(c_max: u32) -> u32 {
    let value = c_max as u64 + 1;
    if value <= 1 {
        0
    } else {
        64 - (value - 1).leading_zeros()
    }
}
