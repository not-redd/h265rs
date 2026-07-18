use super::{BitReader, SyntaxError};

/// Profile and constraint fields common to the general and sub-layer syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileInfo {
    /// `profile_space`.
    pub profile_space: u8,
    /// `tier_flag`.
    pub tier_flag: bool,
    /// `profile_idc`.
    pub profile_idc: u8,
    /// The 32 compatibility flags in table order.
    pub compatibility_flags: [bool; 32],
    /// `progressive_source_flag`.
    pub progressive_source_flag: bool,
    /// `interlaced_source_flag`.
    pub interlaced_source_flag: bool,
    /// `non_packed_constraint_flag`.
    pub non_packed_constraint_flag: bool,
    /// `frame_only_constraint_flag`.
    pub frame_only_constraint_flag: bool,
    /// `general_max_12bit_constraint_flag`, when the profile branch contains it.
    pub max_12bit_constraint_flag: Option<bool>,
    /// `general_max_10bit_constraint_flag`, when the profile branch contains it.
    pub max_10bit_constraint_flag: Option<bool>,
    /// `general_max_8bit_constraint_flag`, when the profile branch contains it.
    pub max_8bit_constraint_flag: Option<bool>,
    /// `general_max_422chroma_constraint_flag`, when present.
    pub max_422chroma_constraint_flag: Option<bool>,
    /// `general_max_420chroma_constraint_flag`, when present.
    pub max_420chroma_constraint_flag: Option<bool>,
    /// `general_max_monochrome_constraint_flag`, when present.
    pub max_monochrome_constraint_flag: Option<bool>,
    /// `general_intra_constraint_flag`, when present.
    pub intra_constraint_flag: Option<bool>,
    /// `general_one_picture_only_constraint_flag`, when present.
    pub one_picture_only_constraint_flag: Option<bool>,
    /// `general_lower_bit_rate_constraint_flag`, when present.
    pub lower_bit_rate_constraint_flag: Option<bool>,
    /// `general_max_14bit_constraint_flag`, when the profile branch contains it.
    pub max_14bit_constraint_flag: Option<bool>,
    /// `general_inbld_flag`, or `None` when a reserved bit occupies its position.
    pub inbld_flag: Option<bool>,
    /// Branch-dependent reserved bits, preserved as an integer.
    pub reserved_zero_bits: Option<u64>,
    /// The branch-dependent 44-bit constraint payload preserved verbatim.
    pub constraint_flags: u64,
}

impl ProfileInfo {
    fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let profile_space = reader.read_u(2)? as u8;
        let tier_flag = reader.read_u(1)? != 0;
        let profile_idc = reader.read_u(5)? as u8;
        let mut compatibility_flags = [false; 32];
        for flag in &mut compatibility_flags {
            *flag = reader.read_u(1)? != 0;
        }
        let progressive_source_flag = reader.read_u(1)? != 0;
        let interlaced_source_flag = reader.read_u(1)? != 0;
        let non_packed_constraint_flag = reader.read_u(1)? != 0;
        let frame_only_constraint_flag = reader.read_u(1)? != 0;
        let high_profile_branch = profile_idc == 4
            || profile_idc == 5
            || profile_idc == 6
            || profile_idc == 7
            || profile_idc == 8
            || profile_idc == 9
            || profile_idc == 10
            || profile_idc == 11
            || compatibility_flags[4]
            || compatibility_flags[5]
            || compatibility_flags[6]
            || compatibility_flags[7]
            || compatibility_flags[8]
            || compatibility_flags[9]
            || compatibility_flags[10]
            || compatibility_flags[11];
        let profile_five_family = profile_idc == 5
            || profile_idc == 9
            || profile_idc == 10
            || profile_idc == 11
            || compatibility_flags[5]
            || compatibility_flags[9]
            || compatibility_flags[10]
            || compatibility_flags[11];
        let profile_two_branch = profile_idc == 2 || compatibility_flags[2];
        let mut constraint_flags = 0u64;
        let mut append_constraint_bits = |value: u64, count: usize| {
            constraint_flags = (constraint_flags << count) | value;
        };
        let (
            max_12bit_constraint_flag,
            max_10bit_constraint_flag,
            max_8bit_constraint_flag,
            max_422chroma_constraint_flag,
            max_420chroma_constraint_flag,
            max_monochrome_constraint_flag,
            intra_constraint_flag,
            one_picture_only_constraint_flag,
            lower_bit_rate_constraint_flag,
            max_14bit_constraint_flag,
            reserved_zero_bits,
        ) = if high_profile_branch {
            let flags = reader.read_u(9)?;
            append_constraint_bits(flags, 9);
            let max_12bit_constraint_flag = Some((flags >> 8) != 0);
            let max_10bit_constraint_flag = Some(((flags >> 7) & 1) != 0);
            let max_8bit_constraint_flag = Some(((flags >> 6) & 1) != 0);
            let max_422chroma_constraint_flag = Some(((flags >> 5) & 1) != 0);
            let max_420chroma_constraint_flag = Some(((flags >> 4) & 1) != 0);
            let max_monochrome_constraint_flag = Some(((flags >> 3) & 1) != 0);
            let intra_constraint_flag = Some(((flags >> 2) & 1) != 0);
            let one_picture_only_constraint_flag = Some(((flags >> 1) & 1) != 0);
            let lower_bit_rate_constraint_flag = Some((flags & 1) != 0);
            let (max_14bit_constraint_flag, reserved_zero_bits) = if profile_five_family {
                let max_14bit = reader.read_u(1)? != 0;
                append_constraint_bits(u64::from(max_14bit), 1);
                let reserved = reader.read_u(33)?;
                append_constraint_bits(reserved, 33);
                (Some(max_14bit), Some(reserved))
            } else {
                let reserved = reader.read_u(34)?;
                append_constraint_bits(reserved, 34);
                (None, Some(reserved))
            };
            (
                max_12bit_constraint_flag,
                max_10bit_constraint_flag,
                max_8bit_constraint_flag,
                max_422chroma_constraint_flag,
                max_420chroma_constraint_flag,
                max_monochrome_constraint_flag,
                intra_constraint_flag,
                one_picture_only_constraint_flag,
                lower_bit_rate_constraint_flag,
                max_14bit_constraint_flag,
                reserved_zero_bits,
            )
        } else if profile_two_branch {
            let reserved_prefix = reader.read_u(7)?;
            append_constraint_bits(reserved_prefix, 7);
            let one_picture_only = reader.read_u(1)? != 0;
            append_constraint_bits(u64::from(one_picture_only), 1);
            let reserved_suffix = reader.read_u(35)?;
            append_constraint_bits(reserved_suffix, 35);
            (
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(one_picture_only),
                None,
                None,
                Some(
                    (reserved_prefix << 35) | (u64::from(one_picture_only) << 34) | reserved_suffix,
                ),
            )
        } else {
            let reserved = reader.read_u(43)?;
            append_constraint_bits(reserved, 43);
            (
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(reserved),
            )
        };
        let inbld_branch = profile_idc == 1
            || profile_idc == 2
            || profile_idc == 3
            || profile_idc == 4
            || profile_idc == 5
            || profile_idc == 9
            || profile_idc == 11
            || compatibility_flags[1]
            || compatibility_flags[2]
            || compatibility_flags[3]
            || compatibility_flags[4]
            || compatibility_flags[5]
            || compatibility_flags[9]
            || compatibility_flags[11];
        let inbld_flag = if inbld_branch {
            let value = reader.read_u(1)? != 0;
            append_constraint_bits(u64::from(value), 1);
            Some(value)
        } else {
            let value = reader.read_u(1)?;
            append_constraint_bits(value, 1);
            None
        };
        Ok(Self {
            profile_space,
            tier_flag,
            profile_idc,
            compatibility_flags,
            progressive_source_flag,
            interlaced_source_flag,
            non_packed_constraint_flag,
            frame_only_constraint_flag,
            max_12bit_constraint_flag,
            max_10bit_constraint_flag,
            max_8bit_constraint_flag,
            max_422chroma_constraint_flag,
            max_420chroma_constraint_flag,
            max_monochrome_constraint_flag,
            intra_constraint_flag,
            one_picture_only_constraint_flag,
            lower_bit_rate_constraint_flag,
            max_14bit_constraint_flag,
            inbld_flag,
            reserved_zero_bits,
            constraint_flags,
        })
    }
}

/// One sub-layer's optional profile and level fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubLayerProfileLevel {
    /// Whether profile fields are present for this sub-layer.
    pub profile_present_flag: bool,
    /// Whether a level field is present for this sub-layer.
    pub level_present_flag: bool,
    /// Profile fields when `profile_present_flag` is true.
    pub profile: Option<ProfileInfo>,
    /// Level IDC when `level_present_flag` is true.
    pub level_idc: Option<u8>,
}

/// Parsed `profile_tier_level()` syntax from §7.3.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileTierLevel {
    /// The `profilePresentFlag` function argument.
    pub profile_present_flag: bool,
    /// General profile and constraints, when present.
    pub general_profile: Option<ProfileInfo>,
    /// `general_level_idc`.
    pub general_level_idc: u8,
    /// Sub-layer syntax in ascending sub-layer order.
    pub sub_layers: Vec<SubLayerProfileLevel>,
    /// The reserved two-bit values between the sub-layer flags and profiles.
    pub reserved_zero_2bits: Vec<u8>,
}

/// Parses `profile_tier_level(profilePresentFlag, maxNumSubLayersMinus1)`.
pub fn parse_profile_tier_level(
    reader: &mut BitReader<'_>,
    profile_present_flag: bool,
    max_num_sub_layers_minus1: u8,
) -> Result<ProfileTierLevel, SyntaxError> {
    if max_num_sub_layers_minus1 > 7 {
        return Err(SyntaxError::InvalidSyntaxValue(
            "maxNumSubLayersMinus1 must be at most 7",
        ));
    }
    let general_profile = if profile_present_flag {
        Some(ProfileInfo::parse(reader)?)
    } else {
        None
    };
    let general_level_idc = reader.read_u(8)? as u8;
    let sub_layer_count = usize::from(max_num_sub_layers_minus1);
    let mut profile_flags = Vec::with_capacity(sub_layer_count);
    let mut level_flags = Vec::with_capacity(sub_layer_count);
    for _ in 0..sub_layer_count {
        profile_flags.push(reader.read_u(1)? != 0);
        level_flags.push(reader.read_u(1)? != 0);
    }
    let mut reserved_zero_2bits = Vec::new();
    if sub_layer_count > 0 {
        for _ in sub_layer_count..8 {
            reserved_zero_2bits.push(reader.read_u(2)? as u8);
        }
    }
    let mut sub_layers = Vec::with_capacity(sub_layer_count);
    for index in 0..sub_layer_count {
        let profile = if profile_flags[index] {
            Some(ProfileInfo::parse(reader)?)
        } else {
            None
        };
        let level_idc = if level_flags[index] {
            Some(reader.read_u(8)? as u8)
        } else {
            None
        };
        sub_layers.push(SubLayerProfileLevel {
            profile_present_flag: profile_flags[index],
            level_present_flag: level_flags[index],
            profile,
            level_idc,
        });
    }
    Ok(ProfileTierLevel {
        profile_present_flag,
        general_profile,
        general_level_idc,
        sub_layers,
        reserved_zero_2bits,
    })
}
