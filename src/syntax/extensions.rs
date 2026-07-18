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

/// SPS screen-content-coding extension syntax from §7.3.2.2.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpsSccExtensionSyntax {
    /// `sps_curr_pic_ref_enabled_flag`.
    pub sps_curr_pic_ref_enabled_flag: bool,
    /// `palette_mode_enabled_flag`.
    pub palette_mode_enabled_flag: bool,
    /// `palette_max_size`, when palette mode is enabled.
    pub palette_max_size: Option<u64>,
    /// `delta_palette_max_predictor_size`, when palette mode is enabled.
    pub delta_palette_max_predictor_size: Option<u64>,
    /// `sps_palette_predictor_initializers_present_flag`, when palette mode is enabled.
    pub sps_palette_predictor_initializers_present_flag: Option<bool>,
    /// `sps_num_palette_predictor_initializers_minus1`, when initializers are present.
    pub sps_num_palette_predictor_initializers_minus1: Option<u64>,
    /// SPS palette initializer values, indexed by component then entry.
    pub palette_predictor_initializers: Vec<Vec<u64>>,
    /// `motion_vector_resolution_control_idc`.
    pub motion_vector_resolution_control_idc: u8,
    /// `intra_boundary_filtering_disabled_flag`.
    pub intra_boundary_filtering_disabled_flag: bool,
}

/// SPS multilayer extension syntax from Annex F.7.3.2.2.4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpsMultilayerExtensionSyntax {
    /// `inter_view_mv_vert_constraint_flag`.
    pub inter_view_mv_vert_constraint_flag: bool,
}

/// SPS 3D extension syntax from Annex I.7.3.2.2.5.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sps3dExtensionSyntax {
    /// `iv_di_mc_enabled_flag` for both depth dimensions.
    pub iv_di_mc_enabled_flag: [bool; 2],
    /// `iv_mv_scal_enabled_flag` for both depth dimensions.
    pub iv_mv_scal_enabled_flag: [bool; 2],
    /// Texture-view fields.
    pub log2_ivmc_sub_pb_size_minus3: u64,
    /// Texture-view fields.
    pub iv_res_pred_enabled_flag: bool,
    /// Texture-view fields.
    pub depth_ref_enabled_flag: bool,
    /// Texture-view fields.
    pub vsp_mc_enabled_flag: bool,
    /// Texture-view fields.
    pub dbbp_enabled_flag: bool,
    /// Depth-view fields.
    pub tex_mc_enabled_flag: bool,
    /// Depth-view fields.
    pub log2_texmc_sub_pb_size_minus3: u64,
    /// Depth-view fields.
    pub intra_contour_enabled_flag: bool,
    /// Depth-view fields.
    pub intra_dc_only_wedge_enabled_flag: bool,
    /// Depth-view fields.
    pub cqt_cu_part_pred_enabled_flag: bool,
    /// Depth-view fields.
    pub inter_dc_only_enabled_flag: bool,
    /// Depth-view fields.
    pub skip_intra_enabled_flag: bool,
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

impl SpsMultilayerExtensionSyntax {
    /// Parses `sps_multilayer_extension()`.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        Ok(Self {
            inter_view_mv_vert_constraint_flag: reader.read_u(1)? != 0,
        })
    }
}

impl Sps3dExtensionSyntax {
    /// Parses `sps_3d_extension()`.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let iv_di_mc_enabled_flag = [reader.read_u(1)? != 0, reader.read_u(1)? != 0];
        let iv_mv_scal_enabled_flag = [reader.read_u(1)? != 0, reader.read_u(1)? != 0];
        let log2_ivmc_sub_pb_size_minus3 = reader.read_ue()?;
        let iv_res_pred_enabled_flag = reader.read_u(1)? != 0;
        let depth_ref_enabled_flag = reader.read_u(1)? != 0;
        let vsp_mc_enabled_flag = reader.read_u(1)? != 0;
        let dbbp_enabled_flag = reader.read_u(1)? != 0;
        let tex_mc_enabled_flag = reader.read_u(1)? != 0;
        let log2_texmc_sub_pb_size_minus3 = reader.read_ue()?;
        let intra_contour_enabled_flag = reader.read_u(1)? != 0;
        let intra_dc_only_wedge_enabled_flag = reader.read_u(1)? != 0;
        let cqt_cu_part_pred_enabled_flag = reader.read_u(1)? != 0;
        let inter_dc_only_enabled_flag = reader.read_u(1)? != 0;
        let skip_intra_enabled_flag = reader.read_u(1)? != 0;
        Ok(Self {
            iv_di_mc_enabled_flag,
            iv_mv_scal_enabled_flag,
            log2_ivmc_sub_pb_size_minus3,
            iv_res_pred_enabled_flag,
            depth_ref_enabled_flag,
            vsp_mc_enabled_flag,
            dbbp_enabled_flag,
            tex_mc_enabled_flag,
            log2_texmc_sub_pb_size_minus3,
            intra_contour_enabled_flag,
            intra_dc_only_wedge_enabled_flag,
            cqt_cu_part_pred_enabled_flag,
            inter_dc_only_enabled_flag,
            skip_intra_enabled_flag,
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
    /// Multilayer extension syntax, when selected.
    pub multilayer_extension: Option<SpsMultilayerExtensionSyntax>,
    /// 3D extension syntax, when selected.
    pub three_d_extension: Option<Sps3dExtensionSyntax>,
    /// SCC extension syntax, when selected.
    pub scc_extension: Option<SpsSccExtensionSyntax>,
    /// `sps_extension_data_flag` values when `sps_extension_4bits` is non-zero.
    pub extension_data: Vec<bool>,
    /// Whether `rbsp_trailing_bits()` was consumed by this parser.
    pub trailing_bits_parsed: bool,
}

fn palette_bit_count(bit_depth_minus8: u64) -> Result<usize, SyntaxError> {
    usize::try_from(
        bit_depth_minus8
            .checked_add(8)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "palette bit depth overflows",
            ))?,
    )
    .ok()
    .filter(|&count| count <= 64)
    .ok_or(SyntaxError::InvalidSyntaxValue(
        "palette initializer bit depth must be at most 64",
    ))
}

impl SpsSccExtensionSyntax {
    fn parse(
        reader: &mut BitReader<'_>,
        chroma_format_idc: u64,
        bit_depth_luma_minus8: u64,
        bit_depth_chroma_minus8: u64,
    ) -> Result<Self, SyntaxError> {
        let sps_curr_pic_ref_enabled_flag = reader.read_u(1)? != 0;
        let palette_mode_enabled_flag = reader.read_u(1)? != 0;
        let (
            palette_max_size,
            delta_palette_max_predictor_size,
            sps_palette_predictor_initializers_present_flag,
            sps_num_palette_predictor_initializers_minus1,
            palette_predictor_initializers,
        ) = if palette_mode_enabled_flag {
            let palette_max_size = reader.read_ue()?;
            let delta_palette_max_predictor_size = reader.read_ue()?;
            let initializers_present = reader.read_u(1)? != 0;
            if initializers_present {
                let count_minus1 = reader.read_ue()?;
                let count = usize::try_from(count_minus1)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "too many SPS palette initializers",
                    ))?;
                let component_count = if chroma_format_idc == 0 { 1 } else { 3 };
                let luma_bits = palette_bit_count(bit_depth_luma_minus8)?;
                let chroma_bits = palette_bit_count(bit_depth_chroma_minus8)?;
                let mut initializers = Vec::with_capacity(component_count);
                for component in 0..component_count {
                    let bit_count = if component == 0 {
                        luma_bits
                    } else {
                        chroma_bits
                    };
                    let mut values = Vec::with_capacity(count);
                    for _ in 0..count {
                        values.push(reader.read_u(bit_count)?);
                    }
                    initializers.push(values);
                }
                (
                    Some(palette_max_size),
                    Some(delta_palette_max_predictor_size),
                    Some(true),
                    Some(count_minus1),
                    initializers,
                )
            } else {
                (
                    Some(palette_max_size),
                    Some(delta_palette_max_predictor_size),
                    Some(false),
                    None,
                    Vec::new(),
                )
            }
        } else {
            (None, None, None, None, Vec::new())
        };
        Ok(Self {
            sps_curr_pic_ref_enabled_flag,
            palette_mode_enabled_flag,
            palette_max_size,
            delta_palette_max_predictor_size,
            sps_palette_predictor_initializers_present_flag,
            sps_num_palette_predictor_initializers_minus1,
            palette_predictor_initializers,
            motion_vector_resolution_control_idc: reader.read_u(2)? as u8,
            intra_boundary_filtering_disabled_flag: reader.read_u(1)? != 0,
        })
    }
}

impl SpsExtensionSyntax {
    /// Parses the SPS extension selectors and extension bodies.
    pub fn parse(
        reader: &mut BitReader<'_>,
        chroma_format_idc: u64,
        bit_depth_luma_minus8: u64,
        bit_depth_chroma_minus8: u64,
    ) -> Result<Self, SyntaxError> {
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
        let multilayer_extension = if sps_multilayer_extension_flag {
            Some(SpsMultilayerExtensionSyntax::parse(reader)?)
        } else {
            None
        };
        let three_d_extension = if sps_3d_extension_flag {
            Some(Sps3dExtensionSyntax::parse(reader)?)
        } else {
            None
        };
        let scc_extension = if sps_scc_extension_flag {
            Some(SpsSccExtensionSyntax::parse(
                reader,
                chroma_format_idc,
                bit_depth_luma_minus8,
                bit_depth_chroma_minus8,
            )?)
        } else {
            None
        };
        let mut extension_data = Vec::new();
        let trailing_bits_parsed = {
            if sps_extension_4bits != 0 {
                while reader.more_rbsp_data() {
                    extension_data.push(reader.read_u(1)? != 0);
                }
            }
            reader.read_rbsp_trailing_bits()?;
            true
        };
        Ok(Self {
            sps_range_extension_flag,
            sps_multilayer_extension_flag,
            sps_3d_extension_flag,
            sps_scc_extension_flag,
            sps_extension_4bits,
            range_extension,
            multilayer_extension,
            three_d_extension,
            scc_extension,
            extension_data,
            trailing_bits_parsed,
        })
    }
}
