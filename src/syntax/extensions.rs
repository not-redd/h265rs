use super::{BitReader, SyntaxError};

/// SPS range-extension syntax from §7.3.2.2.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpsRangeExtensionSyntax {
    /// `transform_skip_rotation_enabled_flag`.
    pub transform_skip_rotation_enabled_flag: bool,
    /// `transform_skip_context_enabled_flag`.
    pub transform_skip_context_enabled_flag: bool,
    /// `implicit_rdpcm_enabled_flag`.
    pub implicit_rdpcm_enabled_flag: bool,
    /// `explicit_rdpcm_enabled_flag`.
    pub explicit_rdpcm_enabled_flag: bool,
    /// `extended_precision_processing_flag`.
    pub extended_precision_processing_flag: bool,
    /// `intra_smoothing_disabled_flag`.
    pub intra_smoothing_disabled_flag: bool,
    /// `high_precision_offsets_enabled_flag`.
    pub high_precision_offsets_enabled_flag: bool,
    /// `persistent_rice_adaptation_enabled_flag`.
    pub persistent_rice_adaptation_enabled_flag: bool,
    /// `cabac_bypass_alignment_enabled_flag`.
    pub cabac_bypass_alignment_enabled_flag: bool,
}

impl SpsRangeExtensionSyntax {
    /// Parses one `sps_range_extension()` syntax structure.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        Ok(Self {
            transform_skip_rotation_enabled_flag: reader.read_u(1)? != 0,
            transform_skip_context_enabled_flag: reader.read_u(1)? != 0,
            implicit_rdpcm_enabled_flag: reader.read_u(1)? != 0,
            explicit_rdpcm_enabled_flag: reader.read_u(1)? != 0,
            extended_precision_processing_flag: reader.read_u(1)? != 0,
            intra_smoothing_disabled_flag: reader.read_u(1)? != 0,
            high_precision_offsets_enabled_flag: reader.read_u(1)? != 0,
            persistent_rice_adaptation_enabled_flag: reader.read_u(1)? != 0,
            cabac_bypass_alignment_enabled_flag: reader.read_u(1)? != 0,
        })
    }
}

/// SPS extension selector syntax from §7.3.2.2.1.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpsExtensionSyntax {
    /// `sps_range_extension_flag`.
    pub sps_range_extension_flag: bool,
    /// `sps_multilayer_extension_flag`.
    pub sps_multilayer_extension_flag: bool,
    /// `sps_3d_extension_flag`.
    pub sps_3d_extension_flag: bool,
    /// `sps_scc_extension_flag`.
    pub sps_scc_extension_flag: bool,
    /// `sps_extension_4bits`.
    pub sps_extension_4bits: u8,
    /// Range-extension syntax, when selected.
    pub range_extension: Option<SpsRangeExtensionSyntax>,
}

impl SpsExtensionSyntax {
    /// Parses the SPS extension selectors and the range extension, if present.
    ///
    /// The reader stops before multilayer, 3D, SCC, and extension-data syntax.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let sps_range_extension_flag = reader.read_u(1)? != 0;
        let sps_multilayer_extension_flag = reader.read_u(1)? != 0;
        let sps_3d_extension_flag = reader.read_u(1)? != 0;
        let sps_scc_extension_flag = reader.read_u(1)? != 0;
        let sps_extension_4bits = reader.read_u(4)? as u8;
        let range_extension = if sps_range_extension_flag {
            Some(SpsRangeExtensionSyntax::parse(reader)?)
        } else {
            None
        };
        Ok(Self {
            sps_range_extension_flag,
            sps_multilayer_extension_flag,
            sps_3d_extension_flag,
            sps_scc_extension_flag,
            sps_extension_4bits,
            range_extension,
        })
    }
}
