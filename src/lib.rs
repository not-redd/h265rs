//! Clause 6 and Clause 7 building blocks for an H.265 decoder.
//!
//! The crate implements the structural, addressing, scan-order and syntax
//! processes described by ITU-T H.265 §§6-7, including parameter sets, slice
//! headers and recursive slice-segment data. Prediction reconstruction remains
//! in the Clause 8 modules, while Clause 7 CABAC branches consume the
//! [`CabacReader`] interface and the Clause 9 arithmetic implementation
//! supplied by [`CabacDecoder`].

#![forbid(unsafe_code)]

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
    parse_coding_quadtree, parse_coding_quadtree_shape, parse_coding_unit,
    parse_coding_unit_with_amp, parse_cross_component_prediction, parse_motion_vector_difference,
    parse_palette_coding, parse_palette_coding_with_bit_depth, parse_pcm_sample,
    parse_pcm_sample_from_cabac, parse_prediction_unit, parse_prediction_unit_with_dimensions,
    parse_residual_coding, parse_residual_coding_for_component,
    parse_residual_coding_for_component_with_options,
    parse_residual_coding_for_component_with_options_and_state, parse_sao,
    parse_sao_with_bit_depth, parse_slice_segment_data, parse_slice_segment_data_with_bit_depth,
    parse_slice_segment_data_with_bit_depth_and_amp, parse_slice_segment_layer_rbsp,
    parse_slice_segment_layer_rbsp_with_bit_depth,
    parse_slice_segment_layer_rbsp_with_bit_depth_and_amp, parse_transform_tree,
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
