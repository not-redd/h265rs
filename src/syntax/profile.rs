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
        let constraint_flags = reader.read_u(44)?;
        Ok(Self {
            profile_space,
            tier_flag,
            profile_idc,
            compatibility_flags,
            progressive_source_flag,
            interlaced_source_flag,
            non_packed_constraint_flag,
            frame_only_constraint_flag,
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
