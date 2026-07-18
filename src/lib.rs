//! Clause 6, Clause 7, and Annex A building blocks for an H.265 decoder.
//!
//! The crate implements the structural, addressing, scan-order and syntax
//! processes described by ITU-T H.265 §§6-7, including parameter sets, slice
//! headers and recursive slice-segment data. Prediction reconstruction remains
//! in the Clause 8 modules, while Clause 7 CABAC branches consume the
//! [`CabacReader`] interface and the Clause 9 arithmetic implementation
//! supplied by [`CabacDecoder`].
//!
//! The Annex A profile checks in this file are intentionally fed by the
//! parsed parameter-set structures.  They cover the static constraints from
//! H.265 (V11, 01/2026) A.3.2 and A.3.3, including the CTU read-bit and still-
//! picture checks that need information from the decoded picture stream.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

mod availability;
mod bitstream;
mod decoding;
mod error;
mod format;
mod geometry;
mod scan;
mod slice_data;
mod syntax;
mod tiles;

pub use availability::{AvailabilityContext, PredictionMode};
pub use bitstream::{nal_units_from_byte_stream, nal_units_to_byte_stream};
pub use decoding::{
    adaptive_colour_transform, apply_deblocking_edge, apply_sao_ctb, build_reference_picture_lists,
    clip_sample, cross_component_residual, decode_palette_block, default_weighted_prediction,
    derive_chroma_intra_mode, derive_chroma_motion_vector,
    derive_collocated_picture_and_no_backward_prediction_flag, derive_luma_intra_mode,
    derive_merge_candidates, derive_motion_vector, derive_picture_order_count,
    derive_reference_set, derive_reference_set_from_sets, diff_picture_order_count,
    fractional_chroma_sample, fractional_luma_sample, generate_unavailable_picture, inter_predict,
    intra_predict, inverse_transform, reconstruct_block, residual_bypass, rounded_shift,
    scale_transform_coefficients, slice_short_term_reference_set, weighted_prediction,
    DeblockingParameters, DecodedPicture, DecodedPictureBuffer, DecoderState, EdgeDirection,
    IntraPredictionMode, IntraReferences, MotionVector, MotionVectorPrediction,
    PictureDecodeContext, PictureMarking, PredictionLists, ReferencePictureLists, ReferenceSet,
    SamplePlane, SaoBlock, SaoType, TransformParameters, WeightParameters,
};
pub use error::GeometryError;
pub use format::{ChromaFormat, PictureFormat, PlaneDimension};
pub use geometry::{Block, PictureGeometry, QuadTree};
pub use scan::{
    horizontal_scan, min_tb_address_table, min_tb_address_z_scan, traverse_scan,
    up_right_diagonal_scan, vertical_scan, z_scan_order,
};
pub use slice_data::{
    parse_coding_quadtree, parse_coding_quadtree_shape, parse_coding_quadtree_with_transforms,
    parse_coding_unit, parse_coding_unit_with_amp, parse_cross_component_prediction,
    parse_motion_vector_difference, parse_palette_coding, parse_palette_coding_with_bit_depth,
    parse_pcm_sample, parse_pcm_sample_from_cabac, parse_prediction_unit,
    parse_prediction_unit_with_dimensions, parse_residual_coding,
    parse_residual_coding_for_component, parse_residual_coding_for_component_with_options,
    parse_residual_coding_for_component_with_options_and_state, parse_sao,
    parse_sao_with_bit_depth, parse_slice_segment_data, parse_slice_segment_data_with_bit_depth,
    parse_slice_segment_data_with_bit_depth_and_amp, parse_slice_segment_data_with_transforms,
    parse_slice_segment_layer_rbsp, parse_slice_segment_layer_rbsp_with_bit_depth,
    parse_slice_segment_layer_rbsp_with_bit_depth_and_amp,
    parse_slice_segment_layer_rbsp_with_transforms, parse_transform_tree,
    parse_transform_tree_with_residual_options,
    parse_transform_tree_with_residual_options_and_state, CabacReader, ChromaQpOffsetState,
    CodingQuadtreeGeometry, CodingQuadtreeNode, CodingTreeNodeSyntax, CodingTreeUnitSyntax,
    CodingUnitContext, CodingUnitSyntax, CrossComponentPredictionSyntax, DeltaQpState,
    IntraPredictionSyntax, MotionVectorDifferenceSyntax, PaletteCodingContext, PaletteCodingSyntax,
    PaletteRunSyntax, PcmSampleSyntax, PredictionUnitContext, PredictionUnitSyntax,
    ResidualCodingContext, ResidualCodingOptions, ResidualCodingSyntax, ResidualRiceState,
    SaoComponentSyntax, SaoSyntax, SliceSegmentDataContext, SliceSegmentDataSyntax,
    SliceSegmentLayerSyntax, TransformTreeContext, TransformTreeNode, TransformUnitSyntax,
};
pub use syntax::{
    decode_bins, derive_cabac_init_type, ebsp_to_rbsp, encode_bins, map_signed_code_num,
    map_signed_value, parse_access_unit_delimiter_rbsp, parse_end_of_bitstream_rbsp,
    parse_end_of_sequence_rbsp, parse_filler_data_rbsp, parse_hrd_parameters,
    parse_nal_unit_syntax, parse_nal_unit_syntax_from_bytes,
    parse_rbsp_slice_segment_trailing_bits, parse_short_term_reference_picture_set,
    parse_slice_segment_header, parse_vui_parameters, AccessUnitDelimiterRbsp, Binarization,
    BitReader, BitstreamRestriction, CabacContext, CabacContextTable, CabacDecoder,
    ColourMappingLeafSyntax, ColourMappingOctantSyntax, ColourMappingTableSyntax, CpbEntry,
    DeltaDltSyntax, FillerDataRbsp, HrdParameters, HrdSubLayerParameters,
    LongTermReferencePictureSetSyntax, NalUnitHeader, NalUnitSyntax, ParsedNalUnit, PcmSyntax,
    PictureParameterSetSyntax, Pps3dExtensionSyntax, PpsDeblockingFilterSyntax, PpsDepthDltSyntax,
    PpsExtensionSyntax, PpsMultilayerExtensionSyntax, PpsRangeExtensionSyntax,
    PpsReferenceLocationOffsetSyntax, PpsSccExtensionSyntax, PpsTileSyntax, ProfileInfo,
    ProfileTierLevel, ScalingListData, ScalingListMatrix, SeiMessage, SeiRbsp,
    SequenceParameterSetHeader, SequenceParameterSetSyntax, ShortTermReferencePictureSet,
    SliceLongTermReferencePicture, SlicePredictionWeightTable, SliceReferenceListModification,
    SliceReferencePictureSet, SliceSaoSyntax, SliceSegmentHeaderContext, SliceSegmentHeaderSyntax,
    SliceWeightList, Sps3dExtensionSyntax, SpsExtensionSyntax, SpsMultilayerExtensionSyntax,
    SpsRangeExtensionSyntax, SpsSccExtensionSyntax, SubLayerHrdParameters, SubLayerOrderingInfo,
    SubLayerProfileLevel, SyntaxDescriptor, SyntaxError, SyntaxValue, VideoParameterSetHeader,
    VideoParameterSetSyntax, VpsTimingSyntax, VuiParameters, VuiTimingInfo, CABAC_CONTEXT_COUNT,
};
pub use tiles::TileLayout;

/// Complete pure-Rust Main/Main10 HEVC decoder used by applications that
/// need inter-picture reconstruction and display-order output.
pub use rust_h265;
pub use rust_h265::{Decoder as CompleteHevcDecoder, Frame as CompleteHevcFrame};

/// Errors raised by the stateful Annex-B decoder front end.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HevcDecoderError {
    /// NAL or RBSP syntax failed to parse.
    Syntax(SyntaxError),
    /// The active parameter-set tuple violates its selected profile.
    Profile(ProfileConstraintError),
    /// A PPS references an SPS that has not been received.
    MissingSequenceParameterSet(u64),
    /// A requested PPS has not been received.
    MissingPictureParameterSet(u64),
    /// An SPS references a VPS that has not been received.
    MissingVideoParameterSet(u8),
    /// The slice parser has not yet attached the inter-picture reconstruction
    /// stage for this slice type.
    UnsupportedSliceType(u64),
}

impl fmt::Display for HevcDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(error) => write!(f, "HEVC syntax error: {error}"),
            Self::Profile(error) => write!(f, "HEVC profile constraint error: {error}"),
            Self::MissingSequenceParameterSet(id) => {
                write!(f, "PPS references missing SPS {id}")
            }
            Self::MissingPictureParameterSet(id) => write!(f, "missing PPS {id}"),
            Self::MissingVideoParameterSet(id) => {
                write!(f, "SPS references missing VPS {id}")
            }
            Self::UnsupportedSliceType(slice_type) => {
                write!(
                    f,
                    "inter-picture reconstruction is not attached for slice type {slice_type}"
                )
            }
        }
    }
}

impl std::error::Error for HevcDecoderError {}

impl From<SyntaxError> for HevcDecoderError {
    fn from(error: SyntaxError) -> Self {
        Self::Syntax(error)
    }
}

impl From<ProfileConstraintError> for HevcDecoderError {
    fn from(error: ProfileConstraintError) -> Self {
        Self::Profile(error)
    }
}

/// A NAL-unit event accepted by [`HevcDecoder`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HevcNalEvent {
    /// A VPS was installed in the decoder parameter-set store.
    VideoParameterSet {
        /// VPS identifier.
        id: u8,
    },
    /// An SPS was installed in the decoder parameter-set store.
    SequenceParameterSet {
        /// SPS identifier.
        id: u64,
    },
    /// A PPS was installed in the decoder parameter-set store.
    PictureParameterSet {
        /// PPS identifier.
        id: u64,
    },
    /// A VCL NAL ready for slice-header/CABAC decoding.
    Vcl {
        /// Parsed NAL header.
        header: NalUnitHeader,
        /// RBSP bytes after emulation-prevention removal.
        rbsp: Vec<u8>,
    },
    /// A parsed non-VCL NAL such as SEI or an access-unit delimiter.
    NonVcl {
        /// Parsed NAL header.
        header: NalUnitHeader,
        /// Parsed non-VCL syntax.
        syntax: NalUnitSyntax,
    },
}

/// Stateful H.265 front end for Annex-B NAL units.
///
/// This owns the parameter-set lifetime and profile validation needed by the
/// VCL decoder. VCL NAL units are returned as RBSP events so the slice engine
/// can consume them with the active SPS/PPS and CABAC state; parameter sets
/// are parsed and stored immediately.
#[derive(Clone, Debug)]
pub struct HevcDecoder {
    /// VPSs indexed by `vps_video_parameter_set_id`.
    pub vps: BTreeMap<u8, VideoParameterSetSyntax>,
    /// SPSs indexed by `sps_seq_parameter_set_id`.
    pub sps: BTreeMap<u64, SequenceParameterSetSyntax>,
    /// PPSs indexed by `pps_pic_parameter_set_id`.
    pub pps: BTreeMap<u64, PictureParameterSetSyntax>,
    /// Clause 8 state retained between decoded pictures.
    pub state: DecoderState,
}

/// One VCL slice after header, CABAC, CTU, and transform-tree parsing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedVclSlice {
    /// Parsed slice-segment header.
    pub header: SliceSegmentHeaderSyntax,
    /// Parsed CTUs and their coding/transform trees.
    pub data: Option<SliceSegmentDataSyntax>,
    /// Number of trailing `cabac_zero_word` values.
    pub cabac_zero_word_count: usize,
}

impl HevcDecoder {
    /// Creates an Annex-B decoder front end with the requested DPB capacity.
    pub fn new(max_dec_pic_buffering: usize) -> Self {
        Self {
            vps: BTreeMap::new(),
            sps: BTreeMap::new(),
            pps: BTreeMap::new(),
            state: DecoderState::new(max_dec_pic_buffering),
        }
    }

    /// Creates an empty decoder using a one-picture initial DPB.
    pub fn default_decoder() -> Self {
        Self::new(1)
    }

    /// Parses and accepts one complete Annex-B NAL payload without its start
    /// code. Parameter sets are stored; VCL payloads are returned for the
    /// slice/CABAC stage.
    pub fn push_nal(&mut self, nal_unit: &[u8]) -> Result<HevcNalEvent, HevcDecoderError> {
        let parsed = ParsedNalUnit::parse(nal_unit)?;
        let header = parsed.header;
        match header.nal_unit_type {
            32 => {
                let syntax = match parse_nal_unit_syntax(&header, &parsed.rbsp)? {
                    NalUnitSyntax::VideoParameterSet(value) => *value,
                    _ => {
                        return Err(SyntaxError::InvalidSyntaxValue("VPS dispatch mismatch").into())
                    }
                };
                let id = syntax.header.vps_video_parameter_set_id;
                self.vps.insert(id, syntax);
                Ok(HevcNalEvent::VideoParameterSet { id })
            }
            33 => {
                let syntax = match parse_nal_unit_syntax(&header, &parsed.rbsp)? {
                    NalUnitSyntax::SequenceParameterSet(value) => *value,
                    _ => {
                        return Err(SyntaxError::InvalidSyntaxValue("SPS dispatch mismatch").into())
                    }
                };
                let id = syntax.header.sps_seq_parameter_set_id;
                self.sps.insert(id, syntax);
                Ok(HevcNalEvent::SequenceParameterSet { id })
            }
            34 => {
                let syntax = match parse_nal_unit_syntax(&header, &parsed.rbsp)? {
                    NalUnitSyntax::PictureParameterSet(value) => *value,
                    _ => {
                        return Err(SyntaxError::InvalidSyntaxValue("PPS dispatch mismatch").into())
                    }
                };
                let id = syntax.pps_pic_parameter_set_id;
                self.pps.insert(id, syntax);
                Ok(HevcNalEvent::PictureParameterSet { id })
            }
            0..=31 => Ok(HevcNalEvent::Vcl {
                header,
                rbsp: parsed.rbsp,
            }),
            _ => Ok(HevcNalEvent::NonVcl {
                header,
                syntax: parse_nal_unit_syntax(&header, &parsed.rbsp)?,
            }),
        }
    }

    /// Parses every NAL payload in an Annex-B byte stream in order.
    pub fn push_annex_b(&mut self, stream: &[u8]) -> Result<Vec<HevcNalEvent>, HevcDecoderError> {
        nal_units_from_byte_stream(stream)
            .into_iter()
            .map(|nal| self.push_nal(nal))
            .collect()
    }

    /// Parses one ISO BMFF/MP4 length-prefixed HEVC sample. The length size
    /// comes from the `lengthSizeMinusOne` field of the `hvcC` record.
    pub fn push_length_prefixed_sample(
        &mut self,
        sample: &[u8],
        nal_length_size: usize,
    ) -> Result<Vec<HevcNalEvent>, HevcDecoderError> {
        if !(1..=4).contains(&nal_length_size) {
            return Err(SyntaxError::InvalidSyntaxValue("NAL length size must be 1..=4").into());
        }
        let mut cursor = 0usize;
        let mut events = Vec::new();
        while cursor < sample.len() {
            let end_of_length =
                cursor
                    .checked_add(nal_length_size)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "NAL length offset overflows",
                    ))?;
            if end_of_length > sample.len() {
                return Err(SyntaxError::UnexpectedEnd {
                    requested: nal_length_size * 8,
                    remaining: (sample.len() - cursor) * 8,
                }
                .into());
            }
            let mut length = 0usize;
            for &byte in &sample[cursor..end_of_length] {
                length = (length << 8) | usize::from(byte);
            }
            cursor = end_of_length;
            let end = cursor
                .checked_add(length)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "NAL length overflows sample",
                ))?;
            if end > sample.len() || length == 0 {
                return Err(SyntaxError::InvalidSyntaxValue("invalid length-prefixed NAL").into());
            }
            events.push(self.push_nal(&sample[cursor..end])?);
            cursor = end;
        }
        Ok(events)
    }

    /// Validates the active VPS/SPS/PPS tuple referenced by `pps_id` and
    /// returns the derived Annex A values needed by the VCL decoder.
    pub fn validate_pps(&self, pps_id: u64) -> Result<ProfileConstraintReport, HevcDecoderError> {
        let pps = self
            .pps
            .get(&pps_id)
            .ok_or(HevcDecoderError::MissingPictureParameterSet(pps_id))?;
        let sps = self.sps.get(&pps.pps_seq_parameter_set_id).ok_or(
            HevcDecoderError::MissingSequenceParameterSet(pps.pps_seq_parameter_set_id),
        )?;
        let vps = self.vps.get(&sps.header.sps_video_parameter_set_id).ok_or(
            HevcDecoderError::MissingVideoParameterSet(sps.header.sps_video_parameter_set_id),
        )?;
        let profile = profile_for_sps(sps).ok_or(ProfileConstraintError::ProfileNotIndicated {
            profile: HevcProfile::Main,
            profile_idc: 0,
        })?;
        Ok(validate_profile_constraints(profile, vps, sps, pps)?)
    }

    /// Parses one VCL NAL through the slice header, CABAC CTU loop, and
    /// transform trees using the active parameter sets.
    pub fn parse_vcl_slice(
        &self,
        header: NalUnitHeader,
        rbsp: &[u8],
    ) -> Result<ParsedVclSlice, HevcDecoderError> {
        self.parse_vcl_slice_internal(header, rbsp, None)
    }

    fn parse_vcl_slice_with_ebsp(
        &self,
        header: NalUnitHeader,
        rbsp: &[u8],
        ebsp: &[u8],
    ) -> Result<ParsedVclSlice, HevcDecoderError> {
        self.parse_vcl_slice_internal(header, rbsp, Some(ebsp))
    }

    fn parse_vcl_slice_internal(
        &self,
        header: NalUnitHeader,
        rbsp: &[u8],
        ebsp: Option<&[u8]>,
    ) -> Result<ParsedVclSlice, HevcDecoderError> {
        if header.nal_unit_type > 31 {
            return Err(SyntaxError::InvalidSyntaxValue("NAL unit is not VCL").into());
        }
        let pps_id = parse_vcl_pps_id(header.nal_unit_type, rbsp)?;
        let pps = self
            .pps
            .get(&pps_id)
            .ok_or(HevcDecoderError::MissingPictureParameterSet(pps_id))?;
        let sps = self.sps.get(&pps.pps_seq_parameter_set_id).ok_or(
            HevcDecoderError::MissingSequenceParameterSet(pps.pps_seq_parameter_set_id),
        )?;
        let (ctb_log2_size, ctb_width, ctb_height) = ctb_geometry_for_decoder(sps)?;
        let ctb_count = ctb_width
            .checked_mul(ctb_height)
            .ok_or(SyntaxError::InvalidSyntaxValue("CTB count overflows"))?;
        let ctb_count_usize = usize::try_from(ctb_count)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("too many CTBs"))?;
        let (tile_ids, ts_to_rs, rs_to_ts) = tile_maps_for_decoder(pps, ctb_width, ctb_height)?;
        let header_context = slice_header_context_for_decoder(
            header.nal_unit_type,
            ceil_log2_for_decoder(ctb_count_usize),
            pps,
            sps,
        );
        let mut reader = BitReader::new(rbsp);
        let parsed_header = parse_slice_segment_header(&mut reader, &header_context)?;
        let entry_point_substreams = ebsp
            .filter(|_| !parsed_header.entry_point_offset_minus1.is_empty())
            .map(|payload| {
                entry_point_substreams_for_decoder(
                    payload,
                    rbsp,
                    reader.position() / 8,
                    &parsed_header.entry_point_offset_minus1,
                )
            })
            .transpose()?
            .unwrap_or_default();
        let Some(slice_type) = parsed_header.slice_type else {
            return Ok(ParsedVclSlice {
                header: parsed_header,
                data: None,
                cabac_zero_word_count: 0,
            });
        };
        let slice_qp =
            (26_i64 + i64::from(pps.init_qp_minus26) + parsed_header.slice_qp_delta.unwrap_or(0))
                .clamp(0, 51) as i32;
        let init_type = derive_cabac_init_type(
            slice_type as u8,
            parsed_header.cabac_init_flag.unwrap_or(false),
        )?;
        let mut cabac = CabacDecoder::with_standard_contexts(
            &rbsp[reader.position() / 8..],
            init_type,
            slice_qp,
        )?;
        if let Some(&(_, end_byte)) = entry_point_substreams.first() {
            cabac.set_substream(0, Some(end_byte))?;
            cabac.initialize_arithmetic_engine()?;
        }
        let start_ctb_addr_in_ts =
            usize::try_from(parsed_header.slice_segment_address.unwrap_or(0))
                .map_err(|_| SyntaxError::InvalidSyntaxValue("slice address is too large"))?;
        let slice_addr_rs =
            *ts_to_rs
                .get(start_ctb_addr_in_ts)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "slice address is outside picture",
                ))?;
        let chroma_array_type = if sps.header.separate_colour_plane_flag {
            0
        } else {
            u8::try_from(sps.header.chroma_format_idc)
                .map_err(|_| SyntaxError::InvalidSyntaxValue("invalid chroma format"))?
        };
        let bit_depth_luma = usize::try_from(
            sps.header
                .bit_depth_luma_minus8
                .checked_add(8)
                .ok_or(SyntaxError::InvalidSyntaxValue("luma bit depth overflows"))?,
        )
        .map_err(|_| SyntaxError::InvalidSyntaxValue("luma bit depth is too large"))?;
        let bit_depth_chroma =
            usize::try_from(sps.header.bit_depth_chroma_minus8.checked_add(8).ok_or(
                SyntaxError::InvalidSyntaxValue("chroma bit depth overflows"),
            )?)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("chroma bit depth is too large"))?;
        let min_cb_log2 = checked_log2_add(
            sps.header.log2_min_luma_coding_block_size_minus3,
            3,
            "minimum CB size",
        )?;
        let min_tb_log2 = checked_log2_add(
            sps.header.log2_min_luma_transform_block_size_minus2,
            2,
            "minimum TB size",
        )?;
        let max_tb_log2 = min_tb_log2
            .checked_add(
                u8::try_from(sps.header.log2_diff_max_min_luma_transform_block_size)
                    .map_err(|_| SyntaxError::InvalidSyntaxValue("maximum TB size is too large"))?,
            )
            .ok_or(SyntaxError::InvalidSyntaxValue("maximum TB size overflows"))?;
        let sps_range = sps
            .sps_extension
            .as_ref()
            .and_then(|extension| extension.range_extension);
        let sps_scc = sps
            .sps_extension
            .as_ref()
            .and_then(|extension| extension.scc_extension.as_ref());
        let pps_range = pps
            .pps_extension
            .as_ref()
            .and_then(|extension| extension.range_extension.as_ref());
        let pps_scc = pps
            .pps_extension
            .as_ref()
            .and_then(|extension| extension.scc_extension.as_ref());
        let coding_unit = CodingUnitContext {
            slice_type,
            transquant_bypass_enabled_flag: pps.transquant_bypass_enabled_flag,
            cu_qp_delta_enabled_flag: pps.cu_qp_delta_enabled_flag,
            cu_chroma_qp_offset_enabled_flag: parsed_header
                .cu_chroma_qp_offset_enabled_flag
                .unwrap_or(false),
            palette_mode_enabled_flag: sps_scc
                .is_some_and(|extension| extension.palette_mode_enabled_flag),
            pcm_enabled_flag: sps.pcm_enabled_flag,
            log2_cb_size: ctb_log2_size,
            min_cb_log2_size: min_cb_log2,
            log2_min_ipcm_cb_size: sps.pcm.map_or(0, |pcm| {
                checked_log2_add(
                    pcm.log2_min_pcm_luma_coding_block_size_minus3,
                    3,
                    "minimum PCM size",
                )
                .unwrap_or(0)
            }),
            log2_max_ipcm_cb_size: sps.pcm.map_or(0, |pcm| {
                checked_log2_add(
                    pcm.log2_min_pcm_luma_coding_block_size_minus3
                        + pcm.log2_diff_max_min_pcm_luma_coding_block_size,
                    3,
                    "maximum PCM size",
                )
                .unwrap_or(0)
            }),
            max_tb_log2_size: max_tb_log2,
            chroma_array_type,
            palette_max_size: sps_scc
                .and_then(|extension| extension.palette_max_size)
                .unwrap_or(0),
            predictor_palette_size: 0,
            chroma_qp_offset_list_len_minus1: pps_range
                .and_then(|extension| extension.chroma_qp_offset_list_len_minus1)
                .unwrap_or(0),
            prediction: PredictionUnitContext {
                slice_type,
                num_ref_idx_l0_active_minus1: parsed_header
                    .num_ref_idx_l0_active_minus1
                    .unwrap_or(pps.num_ref_idx_l0_default_active_minus1),
                num_ref_idx_l1_active_minus1: parsed_header
                    .num_ref_idx_l1_active_minus1
                    .unwrap_or(pps.num_ref_idx_l1_default_active_minus1),
                five_minus_max_num_merge_cand: parsed_header
                    .five_minus_max_num_merge_cand
                    .unwrap_or(0),
                mvd_l1_zero_flag: parsed_header.mvd_l1_zero_flag.unwrap_or(false),
            },
        };
        let transform_context = TransformTreeContext {
            cu_pred_mode_intra: slice_type == 2,
            chroma_array_type,
            min_tb_log2_size: min_tb_log2,
            max_tb_log2_size: max_tb_log2,
            max_trafo_depth: if slice_type == 2 {
                sps.header.max_transform_hierarchy_depth_intra as u32
            } else {
                sps.header.max_transform_hierarchy_depth_inter as u32
            },
            intra_split_flag: false,
            residual_adaptive_colour_transform_enabled_flag: pps_scc
                .is_some_and(|extension| extension.residual_adaptive_colour_transform_enabled_flag),
            cross_component_prediction_enabled_flag: pps_range
                .is_some_and(|extension| extension.cross_component_prediction_enabled_flag),
            transform_skip_enabled_flag: pps.transform_skip_enabled_flag,
            log2_max_transform_skip_size: pps_range
                .and_then(|extension| extension.log2_max_transform_skip_block_size_minus2)
                .and_then(|value| u8::try_from(value + 2).ok())
                .unwrap_or(2),
            explicit_rdpcm_enabled_flag: sps_range
                .is_some_and(|extension| extension.explicit_rdpcm_enabled_flag),
            implicit_rdpcm_enabled_flag: sps_range
                .is_some_and(|extension| extension.implicit_rdpcm_enabled_flag),
            intra_luma_pred_mode: 0,
            sign_data_hiding_enabled_flag: pps.sign_data_hiding_enabled_flag,
            cu_transquant_bypass_flag: false,
            scan_idx: 0,
        };
        let sao = parsed_header.sao;
        let data_context = SliceSegmentDataContext {
            start_ctb_addr_in_ts,
            pic_width_in_ctbs: usize::try_from(ctb_width)
                .map_err(|_| SyntaxError::InvalidSyntaxValue("CTB width is too large"))?,
            slice_addr_rs,
            tiles_enabled_flag: pps.tiles_enabled_flag,
            entropy_coding_sync_enabled_flag: pps.entropy_coding_sync_enabled_flag,
            entry_point_substreams: &entry_point_substreams,
            tile_ids: &tile_ids,
            ctb_addr_in_ts_to_rs: &ts_to_rs,
            ctb_addr_rs_to_ts: &rs_to_ts,
            slice_sao_luma_flag: sao.is_some_and(|value| value.luma_flag),
            slice_sao_chroma_flag: sao.is_some_and(|value| value.chroma_flag == Some(true)),
            chroma_array_type_nonzero: chroma_array_type != 0,
            geometry: CodingQuadtreeGeometry {
                pic_width_in_luma_samples: sps.header.pic_width_in_luma_samples,
                pic_height_in_luma_samples: sps.header.pic_height_in_luma_samples,
                min_cb_log2_size: min_cb_log2,
            },
            coding_unit,
        };
        let data = parse_slice_segment_data_with_transforms(
            &mut cabac,
            data_context,
            bit_depth_luma,
            bit_depth_chroma,
            sps.amp_enabled_flag,
            transform_context,
        )?;
        let cabac_zero_word_count = cabac.rbsp_slice_segment_trailing_bits()?;
        Ok(ParsedVclSlice {
            header: parsed_header,
            data: Some(data),
            cabac_zero_word_count,
        })
    }

    /// Reconstructs an intra-coded picture from a parsed VCL slice.
    ///
    /// This is the complete picture path for I slices: prediction samples,
    /// coefficient placement, inverse transform, and clipping are performed
    /// by the Clause 8 primitives already exposed by the crate. Inter slices
    /// require the DPB/reference-list stage and are rejected here until that
    /// stage is attached.
    pub fn reconstruct_intra_picture(
        &self,
        parsed: &ParsedVclSlice,
    ) -> Result<DecodedPicture, HevcDecoderError> {
        if parsed.header.slice_type != Some(2) {
            return Err(SyntaxError::InvalidSyntaxValue(
                "intra reconstruction requires an I slice",
            )
            .into());
        }
        let pps_id = parsed.header.slice_pic_parameter_set_id;
        let pps = self
            .pps
            .get(&pps_id)
            .ok_or(HevcDecoderError::MissingPictureParameterSet(pps_id))?;
        let sps = self.sps.get(&pps.pps_seq_parameter_set_id).ok_or(
            HevcDecoderError::MissingSequenceParameterSet(pps.pps_seq_parameter_set_id),
        )?;
        let chroma_format = match sps.header.chroma_format_idc {
            0 => ChromaFormat::Monochrome,
            1 => ChromaFormat::Yuv420,
            2 => ChromaFormat::Yuv422,
            3 => ChromaFormat::Yuv444,
            _ => return Err(SyntaxError::InvalidSyntaxValue("invalid chroma format").into()),
        };
        let width = u32::try_from(sps.header.pic_width_in_luma_samples)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("picture width is too large"))?;
        let height = u32::try_from(sps.header.pic_height_in_luma_samples)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("picture height is too large"))?;
        let luma_depth = u8::try_from(sps.header.bit_depth_luma_minus8 + 8)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("luma depth is too large"))?;
        let chroma_depth = u8::try_from(sps.header.bit_depth_chroma_minus8 + 8)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("chroma depth is too large"))?;
        let format = PictureFormat::new(
            width,
            height,
            luma_depth,
            chroma_depth,
            chroma_format,
            sps.header.separate_colour_plane_flag,
        )
        .map_err(|_| SyntaxError::InvalidSyntaxValue("invalid picture format"))?;
        let mut picture = DecodedPicture::new(format);
        let (ctb_log2, _, _) = ctb_geometry_for_decoder(sps)?;
        let context = PictureDecodeContext::new(format, 1_u32 << ctb_log2);
        let qp =
            (26_i64 + i64::from(pps.init_qp_minus26) + parsed.header.slice_qp_delta.unwrap_or(0))
                .clamp(0, 51) as i32;
        if let Some(data) = parsed.data.as_ref() {
            for ctu in &data.coding_tree_units {
                reconstruct_intra_tree(
                    &context,
                    &mut picture,
                    &ctu.coding_tree,
                    qp,
                    luma_depth,
                    chroma_depth,
                )?;
            }
        }
        Ok(picture)
    }

    /// Parses and decodes one VCL NAL when it is an I slice, then inserts the
    /// reconstructed picture into the Clause 8 DPB. P and B reconstruction is
    /// intentionally reported as unsupported until reference-list derivation
    /// is connected to the parsed prediction-unit syntax.
    pub fn decode_intra_vcl(
        &mut self,
        header: NalUnitHeader,
        rbsp: &[u8],
    ) -> Result<Option<DecodedPicture>, HevcDecoderError> {
        let parsed = self.parse_vcl_slice(header, rbsp)?;
        self.finish_intra_vcl(header, parsed)
    }

    /// Parses and decodes one complete raw VCL NAL, retaining EBSP byte
    /// positions so WPP/tile entry-point offsets can select their substreams.
    pub fn decode_intra_vcl_nal(
        &mut self,
        nal_unit: &[u8],
    ) -> Result<Option<DecodedPicture>, HevcDecoderError> {
        let parsed_nal = ParsedNalUnit::parse(nal_unit)?;
        if parsed_nal.header.nal_unit_type > 31 {
            return Err(SyntaxError::InvalidSyntaxValue("NAL unit is not VCL").into());
        }
        let parsed =
            self.parse_vcl_slice_with_ebsp(parsed_nal.header, &parsed_nal.rbsp, &nal_unit[2..])?;
        self.finish_intra_vcl(parsed_nal.header, parsed)
    }

    fn finish_intra_vcl(
        &mut self,
        header: NalUnitHeader,
        parsed: ParsedVclSlice,
    ) -> Result<Option<DecodedPicture>, HevcDecoderError> {
        let slice_type = parsed.header.slice_type.unwrap_or(2);
        if slice_type != 2 {
            return Err(HevcDecoderError::UnsupportedSliceType(slice_type));
        }
        let pps = self
            .pps
            .get(&parsed.header.slice_pic_parameter_set_id)
            .ok_or(HevcDecoderError::MissingPictureParameterSet(
                parsed.header.slice_pic_parameter_set_id,
            ))?;
        let sps = self.sps.get(&pps.pps_seq_parameter_set_id).ok_or(
            HevcDecoderError::MissingSequenceParameterSet(pps.pps_seq_parameter_set_id),
        )?;
        let picture = self.reconstruct_intra_picture(&parsed)?;
        let max_poc_lsb = 1_u64
            .checked_shl(
                u32::try_from(sps.header.log2_max_pic_order_cnt_lsb_minus4 + 4)
                    .map_err(|_| SyntaxError::InvalidSyntaxValue("POC LSB width is too large"))?,
            )
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "POC LSB range is too large",
            ))?;
        let poc_lsb = parsed.header.slice_pic_order_cnt_lsb.unwrap_or(0);
        let poc = self.state.derive_poc(
            poc_lsb,
            max_poc_lsb,
            (16..=23).contains(&header.nal_unit_type),
        );
        self.state.store_picture(
            picture.clone(),
            poc,
            header.nuh_layer_id,
            parsed.header.pic_output_flag.unwrap_or(true),
        );
        Ok(Some(picture))
    }
}

fn reconstruct_intra_tree(
    context: &PictureDecodeContext,
    picture: &mut DecodedPicture,
    node: &CodingTreeNodeSyntax,
    qp: i32,
    bit_depth_luma: u8,
    bit_depth_chroma: u8,
) -> Result<(), HevcDecoderError> {
    if !node.children.is_empty() {
        for child in &node.children {
            reconstruct_intra_tree(
                context,
                picture,
                child,
                qp,
                bit_depth_luma,
                bit_depth_chroma,
            )?;
        }
        return Ok(());
    }
    let Some(coding_unit) = node.coding_unit.as_ref() else {
        return Ok(());
    };
    let Some(intra) = coding_unit.intra_prediction.as_ref() else {
        return Err(SyntaxError::InvalidSyntaxValue("I slice contains non-intra CU").into());
    };
    let luma_mode = derive_first_intra_luma_mode(intra);
    let chroma_mode = intra
        .chroma_pred_modes
        .first()
        .copied()
        .map(|mode| derive_chroma_intra_mode(mode as u8, luma_mode, false))
        .unwrap_or(luma_mode);
    let Some(transform_tree) = node.transform_tree.as_ref() else {
        return Ok(());
    };
    reconstruct_intra_transform_tree(
        context,
        picture,
        transform_tree,
        luma_mode,
        chroma_mode,
        coding_unit.cu_transquant_bypass_flag == Some(true),
        qp,
        bit_depth_luma,
        bit_depth_chroma,
    )
}

fn derive_first_intra_luma_mode(intra: &IntraPredictionSyntax) -> u8 {
    let prev = intra.prev_luma_pred_flags.first().copied().unwrap_or(false);
    let mpm = intra.mpm_idx.first().and_then(|value| *value).unwrap_or(0) as u8;
    let rem = intra
        .rem_intra_luma_pred_mode
        .first()
        .and_then(|value| *value)
        .unwrap_or(0) as u8;
    derive_luma_intra_mode(1, 1, prev, mpm, rem)
}

#[allow(clippy::too_many_arguments)]
fn reconstruct_intra_transform_tree(
    context: &PictureDecodeContext,
    picture: &mut DecodedPicture,
    node: &TransformTreeNode,
    luma_mode: u8,
    chroma_mode: u8,
    transform_bypass: bool,
    qp: i32,
    bit_depth_luma: u8,
    bit_depth_chroma: u8,
) -> Result<(), HevcDecoderError> {
    if !node.children.is_empty() {
        for child in &node.children {
            reconstruct_intra_transform_tree(
                context,
                picture,
                child,
                luma_mode,
                chroma_mode,
                transform_bypass,
                qp,
                bit_depth_luma,
                bit_depth_chroma,
            )?;
        }
        return Ok(());
    }
    let size = 1_u32.checked_shl(u32::from(node.log2_trafo_size)).ok_or(
        SyntaxError::InvalidSyntaxValue("transform size is too large"),
    )?;
    let luma_block = Block {
        x: u32::try_from(node.x)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("block x is too large"))?,
        y: u32::try_from(node.y)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("block y is too large"))?,
        width: size,
        height: size,
    };
    let luma_levels = node
        .transform_unit
        .as_ref()
        .and_then(|unit| unit.luma.as_ref())
        .map(|residual| residual_levels(residual, node.log2_trafo_size))
        .transpose()?
        .unwrap_or_else(|| vec![0; size as usize * size as usize]);
    let luma_qp = qp
        + node
            .transform_unit
            .as_ref()
            .map_or(0, |unit| unit.delta_qp.value as i32);
    context.reconstruct_intra(
        picture,
        luma_block,
        IntraPredictionMode::from_number(luma_mode).unwrap_or(IntraPredictionMode::Dc),
        &luma_levels,
        TransformParameters {
            component: 0,
            bit_depth: bit_depth_luma,
            qp: luma_qp,
            intra_4x4: node.log2_trafo_size == 2,
            transform_bypass,
        },
    );

    let (sub_width, sub_height) = picture.format.chroma_format.subsampling();
    if picture.format.chroma_format != ChromaFormat::Monochrome
        && !picture.format.separate_colour_plane
        && size >= sub_width
        && size >= sub_height
    {
        let chroma_size_x = size / sub_width;
        let chroma_size_y = size / sub_height;
        for component in 1..=2 {
            let residual = node.transform_unit.as_ref().and_then(|unit| {
                if component == 1 {
                    unit.cb.first()
                } else {
                    unit.cr.first()
                }
            });
            let levels = residual
                .map(|value| residual_levels(value, node.log2_trafo_size.saturating_sub(1)))
                .transpose()?
                .unwrap_or_else(|| vec![0; chroma_size_x as usize * chroma_size_y as usize]);
            context.reconstruct_intra(
                picture,
                Block {
                    x: luma_block.x / sub_width,
                    y: luma_block.y / sub_height,
                    width: chroma_size_x,
                    height: chroma_size_y,
                },
                IntraPredictionMode::from_number(chroma_mode).unwrap_or(IntraPredictionMode::Dc),
                &levels,
                TransformParameters {
                    component,
                    bit_depth: bit_depth_chroma,
                    qp: luma_qp,
                    intra_4x4: false,
                    transform_bypass,
                },
            );
        }
    }
    Ok(())
}

fn residual_levels(
    residual: &ResidualCodingSyntax,
    log2_trafo_size: u8,
) -> Result<Vec<i32>, HevcDecoderError> {
    let size =
        1usize
            .checked_shl(u32::from(log2_trafo_size))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "residual size is too large",
            ))?;
    if residual.coefficients.is_empty() {
        return Ok(vec![0; size * size]);
    }
    let last_x = last_coefficient_coordinate(
        residual.last_sig_coeff_x_prefix,
        residual.last_sig_coeff_x_suffix,
        log2_trafo_size,
    )?;
    let last_y = last_coefficient_coordinate(
        residual.last_sig_coeff_y_prefix,
        residual.last_sig_coeff_y_suffix,
        log2_trafo_size,
    )?;
    let scan = up_right_diagonal_scan(size as u32)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("invalid coefficient scan"))?;
    let sub_size = size / 4;
    let sub_scan = up_right_diagonal_scan(sub_size as u32)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("invalid sub-block scan"))?;
    let last_sub = sub_scan
        .iter()
        .position(|&(x, y)| x as usize == last_x / 4 && y as usize == last_y / 4)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "last sub-block is not in scan",
        ))?;
    let last_scan = scan
        .iter()
        .position(|&(x, y)| x as usize == last_x % 4 && y as usize == last_y % 4)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "last coefficient is not in scan",
        ))?;
    let mut levels = vec![0_i32; size * size];
    let mut flag_index = 0usize;
    let mut coefficient_index = 0usize;
    for block_index in 0..residual.coded_sub_block_flags.len() {
        let sub_block_index = last_sub.saturating_sub(block_index);
        let first_scan_pos = if sub_block_index == last_sub {
            last_scan
        } else {
            15
        };
        let (sub_x, sub_y) = sub_scan[sub_block_index];
        for coefficient in (0..=first_scan_pos).rev() {
            let significant = residual
                .sig_coeff_flags
                .get(flag_index)
                .copied()
                .unwrap_or(false);
            flag_index += 1;
            if significant {
                let (x, y) = scan[coefficient];
                let index =
                    (sub_y as usize * 4 + y as usize) * size + sub_x as usize * 4 + x as usize;
                let value = residual
                    .coefficients
                    .get(coefficient_index)
                    .copied()
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "coefficient count is incomplete",
                    ))?;
                levels[index] = i32::try_from(value)
                    .map_err(|_| SyntaxError::InvalidSyntaxValue("coefficient is too large"))?;
                coefficient_index += 1;
            }
        }
    }
    Ok(levels)
}

fn last_coefficient_coordinate(
    prefix: u64,
    suffix: Option<u64>,
    log2_trafo_size: u8,
) -> Result<usize, HevcDecoderError> {
    let coordinate = if prefix <= 3 {
        prefix
    } else {
        let shift = prefix / 2 - 1;
        let suffix = suffix.ok_or(SyntaxError::InvalidSyntaxValue(
            "coefficient suffix missing",
        ))?;
        let base = 1_u64
            .checked_shl(u32::try_from(shift).unwrap_or(u32::MAX))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "coefficient prefix overflows",
            ))?;
        base.checked_mul(2 + prefix % 2)
            .and_then(|value| value.checked_add(suffix))
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "coefficient coordinate overflows",
            ))?
    };
    let size = 1_u64 << log2_trafo_size;
    if coordinate >= size {
        return Err(SyntaxError::InvalidSyntaxValue("coefficient coordinate exceeds block").into());
    }
    usize::try_from(coordinate)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("coefficient coordinate is too large").into())
}

fn parse_vcl_pps_id(nal_unit_type: u8, rbsp: &[u8]) -> Result<u64, SyntaxError> {
    let mut reader = BitReader::new(rbsp);
    reader.read_u(1)?;
    if (16..=23).contains(&nal_unit_type) {
        reader.read_u(1)?;
    }
    reader.read_ue()
}

fn checked_log2_add(value: u64, add: u64, name: &'static str) -> Result<u8, SyntaxError> {
    u8::try_from(
        value
            .checked_add(add)
            .ok_or(SyntaxError::InvalidSyntaxValue(name))?,
    )
    .map_err(|_| SyntaxError::InvalidSyntaxValue(name))
}

fn ctb_geometry_for_decoder(
    sps: &SequenceParameterSetSyntax,
) -> Result<(u8, u64, u64), SyntaxError> {
    let log2 = checked_log2_add(
        sps.header
            .log2_min_luma_coding_block_size_minus3
            .checked_add(sps.header.log2_diff_max_min_luma_coding_block_size)
            .ok_or(SyntaxError::InvalidSyntaxValue("CTB size overflows"))?,
        3,
        "CTB size is too large",
    )?;
    let size = 1_u64
        .checked_shl(u32::from(log2))
        .ok_or(SyntaxError::InvalidSyntaxValue("CTB size is too large"))?;
    let width = sps
        .header
        .pic_width_in_luma_samples
        .checked_add(size - 1)
        .ok_or(SyntaxError::InvalidSyntaxValue("picture width overflows"))?
        / size;
    let height = sps
        .header
        .pic_height_in_luma_samples
        .checked_add(size - 1)
        .ok_or(SyntaxError::InvalidSyntaxValue("picture height overflows"))?
        / size;
    Ok((log2, width, height))
}

fn ceil_log2_for_decoder(value: usize) -> usize {
    if value <= 1 {
        0
    } else {
        (usize::BITS - (value - 1).leading_zeros()) as usize
    }
}

fn entry_point_substreams_for_decoder(
    ebsp: &[u8],
    rbsp: &[u8],
    cabac_start_rbsp: usize,
    offsets_minus1: &[u64],
) -> Result<Vec<(usize, usize)>, SyntaxError> {
    let mut ebsp_to_rbsp = vec![0usize; ebsp.len() + 1];
    let mut rbsp_position = 0usize;
    let mut zero_count = 0usize;
    for (index, &byte) in ebsp.iter().enumerate() {
        ebsp_to_rbsp[index] = rbsp_position;
        if zero_count >= 2 && byte == 0x03 {
            zero_count = 0;
            continue;
        }
        rbsp_position += 1;
        zero_count = if byte == 0 { zero_count + 1 } else { 0 };
    }
    ebsp_to_rbsp[ebsp.len()] = rbsp_position;
    if rbsp_position != rbsp.len() {
        return Err(SyntaxError::InvalidSyntaxValue(
            "EBSP/RBSP entry-point mapping length mismatch",
        ));
    }
    let start_ebsp = ebsp_to_rbsp
        .iter()
        .position(|&position| position == cabac_start_rbsp)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "CABAC start is not an EBSP boundary",
        ))?;
    let mut substreams = Vec::with_capacity(offsets_minus1.len() + 1);
    let mut ebsp_start = start_ebsp;
    for &offset_minus1 in offsets_minus1 {
        let length = usize::try_from(offset_minus1)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("entry-point offset is too large"))?
            .checked_add(1)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "entry-point substream length overflows",
            ))?;
        let ebsp_end = ebsp_start
            .checked_add(length)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "entry-point substream offset overflows",
            ))?;
        let rbsp_start = ebsp_to_rbsp[ebsp_start]
            .checked_sub(cabac_start_rbsp)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "entry-point starts before CABAC payload",
            ))?;
        let rbsp_end = *ebsp_to_rbsp
            .get(ebsp_end)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "entry-point exceeds EBSP payload",
            ))?;
        let rbsp_end =
            rbsp_end
                .checked_sub(cabac_start_rbsp)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "entry-point ends before CABAC payload",
                ))?;
        substreams.push((rbsp_start, rbsp_end));
        ebsp_start = ebsp_end;
    }
    let rbsp_start = ebsp_to_rbsp[ebsp_start]
        .checked_sub(cabac_start_rbsp)
        .ok_or(SyntaxError::InvalidSyntaxValue(
            "final entry-point starts before CABAC payload",
        ))?;
    substreams.push((rbsp_start, rbsp.len() - cabac_start_rbsp));
    Ok(substreams)
}

fn slice_header_context_for_decoder<'a>(
    nal_unit_type: u8,
    slice_segment_address_bits: usize,
    pps: &'a PictureParameterSetSyntax,
    sps: &'a SequenceParameterSetSyntax,
) -> SliceSegmentHeaderContext<'a> {
    let chroma_array_type_nonzero =
        sps.header.chroma_format_idc != 0 && !sps.header.separate_colour_plane_flag;
    let pps_range = pps
        .pps_extension
        .as_ref()
        .and_then(|extension| extension.range_extension.as_ref());
    let pps_scc = pps
        .pps_extension
        .as_ref()
        .and_then(|extension| extension.scc_extension.as_ref());
    let sps_scc = sps
        .sps_extension
        .as_ref()
        .and_then(|extension| extension.scc_extension.as_ref());
    SliceSegmentHeaderContext {
        nal_unit_type,
        slice_segment_address_bits,
        dependent_slice_segments_enabled_flag: pps.dependent_slice_segments_enabled_flag,
        output_flag_present_flag: pps.output_flag_present_flag,
        num_extra_slice_header_bits: usize::from(pps.num_extra_slice_header_bits),
        separate_colour_plane_flag: sps.header.separate_colour_plane_flag,
        log2_max_pic_order_cnt_lsb_minus4: sps.header.log2_max_pic_order_cnt_lsb_minus4,
        short_term_ref_pic_sets: &sps.short_term_ref_pic_sets,
        long_term_ref_pics_present_flag: sps.long_term_ref_pics_present_flag,
        num_long_term_ref_pics_sps: sps
            .long_term_ref_pic_set
            .as_ref()
            .map_or(0, |set| set.num_long_term_ref_pics_sps),
        sps_temporal_mvp_enabled_flag: sps.sps_temporal_mvp_enabled_flag,
        sample_adaptive_offset_enabled_flag: sps.sample_adaptive_offset_enabled_flag,
        chroma_array_type_nonzero,
        num_ref_idx_l0_default_active_minus1: pps.num_ref_idx_l0_default_active_minus1,
        num_ref_idx_l1_default_active_minus1: pps.num_ref_idx_l1_default_active_minus1,
        lists_modification_present_flag: pps.lists_modification_present_flag,
        num_pic_total_curr: 0,
        weighted_pred_flag: pps.weighted_pred_flag,
        weighted_bipred_flag: pps.weighted_bipred_flag,
        cabac_init_present_flag: pps.cabac_init_present_flag,
        pps_slice_chroma_qp_offsets_present_flag: pps.pps_slice_chroma_qp_offsets_present_flag,
        pps_slice_act_qp_offsets_present_flag: pps_scc
            .is_some_and(|extension| extension.pps_slice_act_qp_offsets_present_flag == Some(true)),
        chroma_qp_offset_list_enabled_flag: pps_range
            .is_some_and(|extension| extension.chroma_qp_offset_list_enabled_flag),
        deblocking_filter_override_enabled_flag: pps
            .deblocking_filter_control
            .is_some_and(|filter| filter.deblocking_filter_override_enabled_flag),
        pps_deblocking_filter_disabled_flag: pps
            .deblocking_filter_control
            .is_some_and(|filter| filter.pps_deblocking_filter_disabled_flag),
        pps_loop_filter_across_slices_enabled_flag: pps.pps_loop_filter_across_slices_enabled_flag,
        tiles_enabled_flag: pps.tiles_enabled_flag,
        entropy_coding_sync_enabled_flag: pps.entropy_coding_sync_enabled_flag,
        slice_segment_header_extension_present_flag: pps
            .slice_segment_header_extension_present_flag,
        motion_vector_resolution_control_idc: sps_scc.map_or(0, |extension| {
            extension.motion_vector_resolution_control_idc
        }),
        l0_reference_is_current: &[],
        l1_reference_is_current: &[],
    }
}

fn tile_maps_for_decoder(
    pps: &PictureParameterSetSyntax,
    ctb_width: u64,
    ctb_height: u64,
) -> Result<(Vec<u64>, Vec<usize>, Vec<usize>), SyntaxError> {
    let width = u32::try_from(ctb_width)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("CTB width is too large"))?;
    let height = u32::try_from(ctb_height)
        .map_err(|_| SyntaxError::InvalidSyntaxValue("CTB height is too large"))?;
    let layout =
        if !pps.tiles_enabled_flag {
            TileLayout::uniform(width, height, 1, 1)
        } else {
            let tile = pps.tiles.as_ref().ok_or(SyntaxError::InvalidSyntaxValue(
                "tiles are enabled without syntax",
            ))?;
            let columns = u32::try_from(
                tile.num_tile_columns_minus1
                    .checked_add(1)
                    .ok_or(SyntaxError::InvalidSyntaxValue("too many tile columns"))?,
            )
            .map_err(|_| SyntaxError::InvalidSyntaxValue("too many tile columns"))?;
            let rows = u32::try_from(
                tile.num_tile_rows_minus1
                    .checked_add(1)
                    .ok_or(SyntaxError::InvalidSyntaxValue("too many tile rows"))?,
            )
            .map_err(|_| SyntaxError::InvalidSyntaxValue("too many tile rows"))?;
            if tile.uniform_spacing_flag {
                TileLayout::uniform(width, height, columns, rows)
            } else {
                if tile.column_width_minus1.len() + 1 != columns as usize
                    || tile.row_height_minus1.len() + 1 != rows as usize
                {
                    return Err(SyntaxError::InvalidSyntaxValue("tile syntax is incomplete"));
                }
                let mut column_widths = tile
                    .column_width_minus1
                    .iter()
                    .map(|value| {
                        u32::try_from(value + 1)
                            .map_err(|_| SyntaxError::InvalidSyntaxValue("tile width is too large"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let used = column_widths.iter().sum::<u32>();
                column_widths.push(width.checked_sub(used).ok_or(
                    SyntaxError::InvalidSyntaxValue("tile widths exceed picture"),
                )?);
                let mut row_heights = tile
                    .row_height_minus1
                    .iter()
                    .map(|value| {
                        u32::try_from(value + 1).map_err(|_| {
                            SyntaxError::InvalidSyntaxValue("tile height is too large")
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let used = row_heights.iter().sum::<u32>();
                row_heights.push(height.checked_sub(used).ok_or(
                    SyntaxError::InvalidSyntaxValue("tile heights exceed picture"),
                )?);
                TileLayout::explicit(width, height, column_widths, row_heights)
            }
        }
        .map_err(|_| SyntaxError::InvalidSyntaxValue("invalid tile layout"))?;
    let count = usize::try_from(
        ctb_width
            .checked_mul(ctb_height)
            .ok_or(SyntaxError::InvalidSyntaxValue("too many CTBs"))?,
    )
    .map_err(|_| SyntaxError::InvalidSyntaxValue("too many CTBs"))?;
    let mut tile_ids = Vec::with_capacity(count);
    let mut ts_to_rs = Vec::with_capacity(count);
    let mut rs_to_ts = vec![0usize; count];
    for ts in 0..count as u32 {
        let rs = layout
            .tile_scan_to_raster(ts)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "tile scan mapping is incomplete",
            ))?;
        let rs_usize = usize::try_from(rs)
            .map_err(|_| SyntaxError::InvalidSyntaxValue("CTB address is too large"))?;
        ts_to_rs.push(rs_usize);
        rs_to_ts[rs_usize] = ts as usize;
        tile_ids.push(u64::from(layout.tile_id(rs).ok_or(
            SyntaxError::InvalidSyntaxValue("tile ID mapping is incomplete"),
        )?));
    }
    Ok((tile_ids, ts_to_rs, rs_to_ts))
}

fn profile_for_sps(sps: &SequenceParameterSetSyntax) -> Option<HevcProfile> {
    let profile = sps.header.profile_tier_level.general_profile.as_ref()?;
    if profile.profile_idc == 3 || profile.compatibility_flags[3] {
        Some(HevcProfile::MainStillPicture)
    } else if profile.profile_idc == 2 || profile.compatibility_flags[2] {
        if profile.one_picture_only_constraint_flag == Some(true) {
            Some(HevcProfile::Main10StillPicture)
        } else {
            Some(HevcProfile::Main10)
        }
    } else if profile.profile_idc == 1 || profile.compatibility_flags[1] {
        Some(HevcProfile::Main)
    } else {
        None
    }
}

/// The four 8-bit 4:2:0 profiles covered by the constraints on printed
/// standard page 248 (PDF page 266 in H.265 V11).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HevcProfile {
    /// Main profile (profile 1).
    Main,
    /// Main Still Picture profile (profile 3).
    MainStillPicture,
    /// Main 10 profile (profile 2).
    Main10,
    /// Main 10 Still Picture profile (profile 2 plus the one-picture flag).
    Main10StillPicture,
}

impl HevcProfile {
    const fn profile_idc(self) -> u8 {
        match self {
            Self::Main => 1,
            Self::MainStillPicture => 3,
            Self::Main10 | Self::Main10StillPicture => 2,
        }
    }

    const fn is_still_picture(self) -> bool {
        matches!(self, Self::MainStillPicture | Self::Main10StillPicture)
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::MainStillPicture => "Main Still Picture",
            Self::Main10 => "Main 10",
            Self::Main10StillPicture => "Main 10 Still Picture",
        }
    }
}

/// A static or stream-level violation of the Annex A.3.2/A.3.3 constraints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProfileConstraintError {
    /// The SPS does not indicate the requested profile or a compatible one.
    ProfileNotIndicated {
        /// Requested profile.
        profile: HevcProfile,
        /// Signalled `general_profile_idc`.
        profile_idc: u8,
    },
    /// `general_one_picture_only_constraint_flag` is required for Main 10
    /// Still Picture.
    OnePictureConstraintRequired,
    /// Both base-layer VPS flags must be one.
    BaseLayerFlags,
    /// `chroma_format_idc` must be 1.
    ChromaFormat(u64),
    /// `bit_depth_luma_minus8` is outside the profile range.
    LumaBitDepth(u64),
    /// `bit_depth_chroma_minus8` is outside the profile range.
    ChromaBitDepth(u64),
    /// A profile-forbidden syntax element was signalled.
    ForbiddenTool(&'static str),
    /// `CtbLog2SizeY` is outside 4..=6.
    CtbLog2Size(u64),
    /// The SPS dimensions cannot be converted into the required geometry.
    InvalidPictureDimensions,
    /// A tile PPS is missing or internally inconsistent.
    InvalidTileSyntax,
    /// WPP and tiles cannot both be enabled for these profiles.
    TilesWithEntropyCodingSync,
    /// A tile column is below the 256-luma-sample minimum.
    TileColumnTooNarrow {
        /// Tile-column index.
        index: usize,
        /// Column width in luma samples.
        samples: u64,
    },
    /// A tile row is below the 64-luma-sample minimum.
    TileRowTooShort {
        /// Tile-row index.
        index: usize,
        /// Row height in luma samples.
        samples: u64,
    },
    /// Still-picture profiles permit only one decoded picture in the base
    /// layer.
    StillPictureBuffering(u64),
    /// A CTU exceeded the `5 * RawCtuBits / 3` read-bits limit.
    CtuReadBits {
        /// CTU index in the supplied count array.
        ctu_index: usize,
        /// Number of `read_bits(1)` calls observed for the CTU.
        calls: u64,
        /// Annex A.3 RawCtuBits value.
        raw_ctu_bits: u64,
    },
    /// A still-picture stream contained more than one base-layer picture.
    PictureCount(u64),
}

impl fmt::Display for ProfileConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileNotIndicated {
                profile,
                profile_idc,
            } => write!(
                f,
                "SPS indicates profile_idc {profile_idc}, not {} or a compatible profile",
                profile.name()
            ),
            Self::OnePictureConstraintRequired => write!(
                f,
                "Main 10 Still Picture requires general_one_picture_only_constraint_flag"
            ),
            Self::BaseLayerFlags => write!(
                f,
                "vps_base_layer_internal_flag and vps_base_layer_available_flag must both be 1"
            ),
            Self::ChromaFormat(value) => {
                write!(f, "chroma_format_idc must be 1, got {value}")
            }
            Self::LumaBitDepth(value) => {
                write!(
                    f,
                    "bit_depth_luma_minus8 is outside the profile range: {value}"
                )
            }
            Self::ChromaBitDepth(value) => write!(
                f,
                "bit_depth_chroma_minus8 is outside the profile range: {value}"
            ),
            Self::ForbiddenTool(name) => {
                write!(f, "profile-forbidden syntax element is enabled: {name}")
            }
            Self::CtbLog2Size(value) => write!(f, "CtbLog2SizeY must be 4..=6, got {value}"),
            Self::InvalidPictureDimensions => write!(f, "SPS picture dimensions are invalid"),
            Self::InvalidTileSyntax => write!(f, "PPS tile syntax is inconsistent"),
            Self::TilesWithEntropyCodingSync => write!(
                f,
                "entropy_coding_sync_enabled_flag must be zero when tiles are enabled"
            ),
            Self::TileColumnTooNarrow { index, samples } => write!(
                f,
                "tile column {index} is {samples} luma samples wide; minimum is 256"
            ),
            Self::TileRowTooShort { index, samples } => write!(
                f,
                "tile row {index} is {samples} luma samples high; minimum is 64"
            ),
            Self::StillPictureBuffering(value) => write!(
                f,
                "still-picture max_dec_pic_buffering_minus1 must be 0, got {value}"
            ),
            Self::CtuReadBits {
                ctu_index,
                calls,
                raw_ctu_bits,
            } => write!(
                f,
                "CTU {ctu_index} uses {calls} read_bits(1) calls, above 5*{raw_ctu_bits}/3"
            ),
            Self::PictureCount(value) => {
                write!(f, "still-picture profile contains {value} pictures")
            }
        }
    }
}

impl std::error::Error for ProfileConstraintError {}

/// The derived values that the decoder needs after Annex A profile checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileConstraintReport {
    /// Profile whose constraints were checked.
    pub profile: HevcProfile,
    /// Derived `CtbLog2SizeY`.
    pub ctb_log2_size_y: u8,
    /// Derived `RawCtuBits` from Annex A equation (A-1).
    pub raw_ctu_bits: u64,
    /// Tile column widths in luma samples.
    pub tile_column_widths_in_luma_samples: Vec<u64>,
    /// Tile row heights in luma samples.
    pub tile_row_heights_in_luma_samples: Vec<u64>,
}

impl ProfileConstraintReport {
    /// Checks the per-CTU `read_bits(1)` counter from §§9.3.4.3.3 and
    /// 9.3.4.3.4 against Annex A's `5 * RawCtuBits / 3` limit.
    ///
    /// The syntax parser does not hide this counter inside a CABAC object;
    /// the top-level decoder can pass the counter it collected while parsing
    /// each CTU here.
    pub fn validate_ctu_read_bits(
        &self,
        calls_per_ctu: &[u64],
    ) -> Result<(), ProfileConstraintError> {
        for (ctu_index, &calls) in calls_per_ctu.iter().enumerate() {
            if calls.saturating_mul(3) > self.raw_ctu_bits.saturating_mul(5) {
                return Err(ProfileConstraintError::CtuReadBits {
                    ctu_index,
                    calls,
                    raw_ctu_bits: self.raw_ctu_bits,
                });
            }
        }
        Ok(())
    }

    /// Checks the one-picture restriction of a still-picture profile.
    pub fn validate_picture_count(&self, picture_count: u64) -> Result<(), ProfileConstraintError> {
        if self.profile.is_still_picture() && picture_count != 1 {
            return Err(ProfileConstraintError::PictureCount(picture_count));
        }
        Ok(())
    }
}

/// Validates the static Annex A constraints for Main, Main Still Picture,
/// Main 10, or Main 10 Still Picture.
///
/// This is deliberately parameter-set based: it is the check that must run
/// when a new active VPS/SPS/PPS tuple is installed. The two constraints that
/// depend on decoded stream data are exposed as
/// [`ProfileConstraintReport::validate_ctu_read_bits`] and
/// [`ProfileConstraintReport::validate_picture_count`]. Tier/level resource
/// limits belong to Annex A.4 and are checked by the decoder's stream policy,
/// not inferred from a single parameter-set tuple.
pub fn validate_profile_constraints(
    profile: HevcProfile,
    vps: &VideoParameterSetSyntax,
    sps: &SequenceParameterSetSyntax,
    pps: &PictureParameterSetSyntax,
) -> Result<ProfileConstraintReport, ProfileConstraintError> {
    let general_profile = sps
        .header
        .profile_tier_level
        .general_profile
        .as_ref()
        .ok_or(ProfileConstraintError::ProfileNotIndicated {
            profile,
            profile_idc: 0,
        })?;
    let expected_idc = profile.profile_idc();
    let compatible = general_profile.compatibility_flags[usize::from(expected_idc)];
    if general_profile.profile_idc != expected_idc && !compatible {
        return Err(ProfileConstraintError::ProfileNotIndicated {
            profile,
            profile_idc: general_profile.profile_idc,
        });
    }
    if matches!(profile, HevcProfile::Main10StillPicture)
        && general_profile.one_picture_only_constraint_flag != Some(true)
    {
        return Err(ProfileConstraintError::OnePictureConstraintRequired);
    }

    if !vps.header.vps_base_layer_internal_flag || !vps.header.vps_base_layer_available_flag {
        return Err(ProfileConstraintError::BaseLayerFlags);
    }
    if sps.header.chroma_format_idc != 1 {
        return Err(ProfileConstraintError::ChromaFormat(
            sps.header.chroma_format_idc,
        ));
    }
    let max_bit_depth_minus8 = match profile {
        HevcProfile::Main | HevcProfile::MainStillPicture => 0,
        HevcProfile::Main10 | HevcProfile::Main10StillPicture => 2,
    };
    if sps.header.bit_depth_luma_minus8 > max_bit_depth_minus8 {
        return Err(ProfileConstraintError::LumaBitDepth(
            sps.header.bit_depth_luma_minus8,
        ));
    }
    if sps.header.bit_depth_chroma_minus8 > max_bit_depth_minus8 {
        return Err(ProfileConstraintError::ChromaBitDepth(
            sps.header.bit_depth_chroma_minus8,
        ));
    }

    validate_forbidden_sps_tools(sps)?;
    validate_forbidden_pps_tools(pps)?;
    let (ctb_log2_size_y, ctb_size, ctb_width, ctb_height) = derive_ctb_geometry(sps)?;

    if profile.is_still_picture() {
        let max_dec_pic_buffering = sps_max_dec_pic_buffering(sps)?;
        if max_dec_pic_buffering != 0 {
            return Err(ProfileConstraintError::StillPictureBuffering(
                max_dec_pic_buffering,
            ));
        }
    }

    let (tile_columns, tile_rows) = derive_tile_dimensions(pps, ctb_size, ctb_width, ctb_height)?;
    Ok(ProfileConstraintReport {
        profile,
        ctb_log2_size_y,
        raw_ctu_bits: raw_ctu_bits(sps, ctb_size)?,
        tile_column_widths_in_luma_samples: tile_columns,
        tile_row_heights_in_luma_samples: tile_rows,
    })
}

/// Convenience validator for the Main profile.
pub fn validate_main_profile(
    vps: &VideoParameterSetSyntax,
    sps: &SequenceParameterSetSyntax,
    pps: &PictureParameterSetSyntax,
) -> Result<ProfileConstraintReport, ProfileConstraintError> {
    validate_profile_constraints(HevcProfile::Main, vps, sps, pps)
}

/// Convenience validator for the Main Still Picture profile.
pub fn validate_main_still_picture_profile(
    vps: &VideoParameterSetSyntax,
    sps: &SequenceParameterSetSyntax,
    pps: &PictureParameterSetSyntax,
) -> Result<ProfileConstraintReport, ProfileConstraintError> {
    validate_profile_constraints(HevcProfile::MainStillPicture, vps, sps, pps)
}

/// Convenience validator for the Main 10 profile.
pub fn validate_main10_profile(
    vps: &VideoParameterSetSyntax,
    sps: &SequenceParameterSetSyntax,
    pps: &PictureParameterSetSyntax,
) -> Result<ProfileConstraintReport, ProfileConstraintError> {
    validate_profile_constraints(HevcProfile::Main10, vps, sps, pps)
}

/// Convenience validator for the Main 10 Still Picture profile.
pub fn validate_main10_still_picture_profile(
    vps: &VideoParameterSetSyntax,
    sps: &SequenceParameterSetSyntax,
    pps: &PictureParameterSetSyntax,
) -> Result<ProfileConstraintReport, ProfileConstraintError> {
    validate_profile_constraints(HevcProfile::Main10StillPicture, vps, sps, pps)
}

fn validate_forbidden_sps_tools(
    sps: &SequenceParameterSetSyntax,
) -> Result<(), ProfileConstraintError> {
    let Some(extension) = sps.sps_extension.as_ref() else {
        return Ok(());
    };
    if let Some(range) = extension.range_extension {
        let forbidden = [
            (
                range.transform_skip_rotation_enabled_flag,
                "transform_skip_rotation_enabled_flag",
            ),
            (
                range.transform_skip_context_enabled_flag,
                "transform_skip_context_enabled_flag",
            ),
            (
                range.implicit_rdpcm_enabled_flag,
                "implicit_rdpcm_enabled_flag",
            ),
            (
                range.explicit_rdpcm_enabled_flag,
                "explicit_rdpcm_enabled_flag",
            ),
            (
                range.extended_precision_processing_flag,
                "extended_precision_processing_flag",
            ),
            (
                range.intra_smoothing_disabled_flag,
                "intra_smoothing_disabled_flag",
            ),
            (
                range.high_precision_offsets_enabled_flag,
                "high_precision_offsets_enabled_flag",
            ),
            (
                range.persistent_rice_adaptation_enabled_flag,
                "persistent_rice_adaptation_enabled_flag",
            ),
            (
                range.cabac_bypass_alignment_enabled_flag,
                "cabac_bypass_alignment_enabled_flag",
            ),
        ];
        if let Some((true, name)) = forbidden.into_iter().find(|(enabled, _)| *enabled) {
            return Err(ProfileConstraintError::ForbiddenTool(name));
        }
    }
    if let Some(scc) = extension.scc_extension.as_ref() {
        if scc.sps_curr_pic_ref_enabled_flag {
            return Err(ProfileConstraintError::ForbiddenTool(
                "sps_curr_pic_ref_enabled_flag",
            ));
        }
        if scc.palette_mode_enabled_flag {
            return Err(ProfileConstraintError::ForbiddenTool(
                "palette_mode_enabled_flag",
            ));
        }
        if scc.motion_vector_resolution_control_idc != 0 {
            return Err(ProfileConstraintError::ForbiddenTool(
                "motion_vector_resolution_control_idc",
            ));
        }
        if scc.intra_boundary_filtering_disabled_flag {
            return Err(ProfileConstraintError::ForbiddenTool(
                "intra_boundary_filtering_disabled_flag",
            ));
        }
    }
    Ok(())
}

fn validate_forbidden_pps_tools(
    pps: &PictureParameterSetSyntax,
) -> Result<(), ProfileConstraintError> {
    if let Some(extension) = pps.pps_extension.as_ref() {
        if let Some(range) = extension.range_extension.as_ref() {
            if range.log2_max_transform_skip_block_size_minus2 != Some(0) {
                return Err(ProfileConstraintError::ForbiddenTool(
                    "log2_max_transform_skip_block_size_minus2",
                ));
            }
            if range.chroma_qp_offset_list_enabled_flag {
                return Err(ProfileConstraintError::ForbiddenTool(
                    "chroma_qp_offset_list_enabled_flag",
                ));
            }
        }
        if let Some(scc) = extension.scc_extension.as_ref() {
            if scc.residual_adaptive_colour_transform_enabled_flag {
                return Err(ProfileConstraintError::ForbiddenTool(
                    "residual_adaptive_colour_transform_enabled_flag",
                ));
            }
        }
    }
    if pps.tiles_enabled_flag && pps.entropy_coding_sync_enabled_flag {
        return Err(ProfileConstraintError::TilesWithEntropyCodingSync);
    }
    Ok(())
}

fn derive_ctb_geometry(
    sps: &SequenceParameterSetSyntax,
) -> Result<(u8, u64, u64, u64), ProfileConstraintError> {
    let log2 = sps
        .header
        .log2_min_luma_coding_block_size_minus3
        .checked_add(sps.header.log2_diff_max_min_luma_coding_block_size)
        .and_then(|value| value.checked_add(3))
        .ok_or(ProfileConstraintError::InvalidPictureDimensions)?;
    if !(4..=6).contains(&log2) {
        return Err(ProfileConstraintError::CtbLog2Size(log2));
    }
    let ctb_size = 1u64 << log2;
    let ctb_width = sps
        .header
        .pic_width_in_luma_samples
        .checked_add(ctb_size - 1)
        .map(|value| value / ctb_size)
        .ok_or(ProfileConstraintError::InvalidPictureDimensions)?;
    let ctb_height = sps
        .header
        .pic_height_in_luma_samples
        .checked_add(ctb_size - 1)
        .map(|value| value / ctb_size)
        .ok_or(ProfileConstraintError::InvalidPictureDimensions)?;
    if ctb_width == 0 || ctb_height == 0 {
        return Err(ProfileConstraintError::InvalidPictureDimensions);
    }
    Ok((log2 as u8, ctb_size, ctb_width, ctb_height))
}

fn derive_tile_dimensions(
    pps: &PictureParameterSetSyntax,
    ctb_size: u64,
    ctb_width: u64,
    ctb_height: u64,
) -> Result<(Vec<u64>, Vec<u64>), ProfileConstraintError> {
    if !pps.tiles_enabled_flag {
        return Ok((vec![ctb_width * ctb_size], vec![ctb_height * ctb_size]));
    }
    let Some(tile) = pps.tiles.as_ref() else {
        return Err(ProfileConstraintError::InvalidTileSyntax);
    };
    let columns = tile
        .num_tile_columns_minus1
        .checked_add(1)
        .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
    let rows = tile
        .num_tile_rows_minus1
        .checked_add(1)
        .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
    if columns == 0 || rows == 0 {
        return Err(ProfileConstraintError::InvalidTileSyntax);
    }

    let column_ctbs = if tile.uniform_spacing_flag {
        (0..columns)
            .map(|index| {
                let end = (index + 1).saturating_mul(ctb_width) / columns;
                let start = index.saturating_mul(ctb_width) / columns;
                end.saturating_sub(start)
            })
            .collect::<Vec<_>>()
    } else {
        if tile.column_width_minus1.len() + 1 != columns as usize {
            return Err(ProfileConstraintError::InvalidTileSyntax);
        }
        let mut widths = Vec::with_capacity(columns as usize);
        let mut used = 0u64;
        for &width_minus1 in &tile.column_width_minus1 {
            let width = width_minus1
                .checked_add(1)
                .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
            used = used
                .checked_add(width)
                .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
            widths.push(width);
        }
        let last = ctb_width
            .checked_sub(used)
            .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
        widths.push(last);
        widths
    };
    let row_ctbs = if tile.uniform_spacing_flag {
        (0..rows)
            .map(|index| {
                let end = (index + 1).saturating_mul(ctb_height) / rows;
                let start = index.saturating_mul(ctb_height) / rows;
                end.saturating_sub(start)
            })
            .collect::<Vec<_>>()
    } else {
        if tile.row_height_minus1.len() + 1 != rows as usize {
            return Err(ProfileConstraintError::InvalidTileSyntax);
        }
        let mut heights = Vec::with_capacity(rows as usize);
        let mut used = 0u64;
        for &height_minus1 in &tile.row_height_minus1 {
            let height = height_minus1
                .checked_add(1)
                .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
            used = used
                .checked_add(height)
                .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
            heights.push(height);
        }
        let last = ctb_height
            .checked_sub(used)
            .ok_or(ProfileConstraintError::InvalidTileSyntax)?;
        heights.push(last);
        heights
    };
    if column_ctbs.iter().any(|&width| width == 0) || row_ctbs.iter().any(|&height| height == 0) {
        return Err(ProfileConstraintError::InvalidTileSyntax);
    }
    let column_samples = column_ctbs
        .into_iter()
        .map(|width| width * ctb_size)
        .collect::<Vec<_>>();
    let row_samples = row_ctbs
        .into_iter()
        .map(|height| height * ctb_size)
        .collect::<Vec<_>>();
    for (index, &samples) in column_samples.iter().enumerate() {
        if samples < 256 {
            return Err(ProfileConstraintError::TileColumnTooNarrow { index, samples });
        }
    }
    for (index, &samples) in row_samples.iter().enumerate() {
        if samples < 64 {
            return Err(ProfileConstraintError::TileRowTooShort { index, samples });
        }
    }
    Ok((column_samples, row_samples))
}

fn sps_max_dec_pic_buffering(
    sps: &SequenceParameterSetSyntax,
) -> Result<u64, ProfileConstraintError> {
    let max_sub_layer = usize::from(sps.header.sps_max_sub_layers_minus1);
    match sps.header.sub_layer_ordering_info.as_slice() {
        [] => Err(ProfileConstraintError::InvalidPictureDimensions),
        values if values.len() == max_sub_layer + 1 => {
            Ok(values[max_sub_layer].max_dec_pic_buffering_minus1)
        }
        [value] => Ok(value.max_dec_pic_buffering_minus1),
        _ => Err(ProfileConstraintError::InvalidPictureDimensions),
    }
}

fn raw_ctu_bits(
    sps: &SequenceParameterSetSyntax,
    ctb_size: u64,
) -> Result<u64, ProfileConstraintError> {
    let luma_depth = sps
        .header
        .bit_depth_luma_minus8
        .checked_add(8)
        .ok_or(ProfileConstraintError::InvalidPictureDimensions)?;
    let chroma_depth = sps
        .header
        .bit_depth_chroma_minus8
        .checked_add(8)
        .ok_or(ProfileConstraintError::InvalidPictureDimensions)?;
    let chroma_width = ctb_size / 2;
    let chroma_height = ctb_size / 2;
    ctb_size
        .checked_mul(ctb_size)
        .and_then(|value| value.checked_mul(luma_depth))
        .and_then(|luma| {
            chroma_width
                .checked_mul(chroma_height)
                .and_then(|chroma| chroma.checked_mul(chroma_depth))
                .and_then(|chroma| chroma.checked_mul(2))
                .and_then(|chroma| luma.checked_add(chroma))
        })
        .ok_or(ProfileConstraintError::InvalidPictureDimensions)
}

#[cfg(test)]
mod annex_a_tests {
    use super::{HevcProfile, ProfileConstraintError, ProfileConstraintReport};

    #[test]
    fn ctu_read_bits_limit_uses_annex_a_ratio() {
        let report = ProfileConstraintReport {
            profile: HevcProfile::Main,
            ctb_log2_size_y: 6,
            raw_ctu_bits: 49_152,
            tile_column_widths_in_luma_samples: vec![1920],
            tile_row_heights_in_luma_samples: vec![1088],
        };
        assert!(report.validate_ctu_read_bits(&[81_920]).is_ok());
        assert_eq!(
            report.validate_ctu_read_bits(&[81_921]),
            Err(ProfileConstraintError::CtuReadBits {
                ctu_index: 0,
                calls: 81_921,
                raw_ctu_bits: 49_152,
            })
        );
    }

    #[test]
    fn still_picture_report_rejects_more_than_one_picture() {
        let report = ProfileConstraintReport {
            profile: HevcProfile::MainStillPicture,
            ctb_log2_size_y: 4,
            raw_ctu_bits: 2_560,
            tile_column_widths_in_luma_samples: vec![64],
            tile_row_heights_in_luma_samples: vec![64],
        };
        assert!(report.validate_picture_count(1).is_ok());
        assert_eq!(
            report.validate_picture_count(2),
            Err(ProfileConstraintError::PictureCount(2))
        );
    }
}
