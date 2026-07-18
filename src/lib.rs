//! Clause 6 and Clause 7 building blocks for an H.265 decoder.
//!
//! The crate implements the structural, addressing, scan-order and syntax
//! processes described by ITU-T H.265 §§6-7, including parameter sets, slice
//! headers and recursive slice-segment data. Prediction reconstruction and
//! the arithmetic decoder itself remain outside this syntax-layer crate; the
//! Clause 7 CABAC branches consume the [`CabacReader`] interface supplied by
//! a Clause 9 implementation.

#![forbid(unsafe_code)]

mod availability;
mod bitstream;
mod error;
mod format;
mod geometry;
mod scan;
mod slice_data;
mod syntax;
mod tiles;

pub use availability::{AvailabilityContext, PredictionMode};
pub use bitstream::{nal_units_from_byte_stream, nal_units_to_byte_stream};
pub use error::GeometryError;
pub use format::{ChromaFormat, PictureFormat, PlaneDimension};
pub use geometry::{Block, PictureGeometry, QuadTree};
pub use scan::{
    horizontal_scan, min_tb_address_table, min_tb_address_z_scan, traverse_scan,
    up_right_diagonal_scan, vertical_scan, z_scan_order,
};
pub use slice_data::{
    parse_coding_quadtree, parse_coding_quadtree_shape, parse_coding_unit,
    parse_cross_component_prediction, parse_motion_vector_difference, parse_palette_coding,
    parse_pcm_sample, parse_prediction_unit, parse_residual_coding, parse_sao,
    parse_slice_segment_data, parse_slice_segment_layer_rbsp, parse_transform_tree, CabacReader,
    ChromaQpOffsetState, CodingQuadtreeGeometry, CodingQuadtreeNode, CodingTreeNodeSyntax,
    CodingTreeUnitSyntax, CodingUnitContext, CodingUnitSyntax, CrossComponentPredictionSyntax,
    DeltaQpState, IntraPredictionSyntax, MotionVectorDifferenceSyntax, PaletteCodingContext,
    PaletteCodingSyntax, PaletteRunSyntax, PcmSampleSyntax, PredictionUnitContext,
    PredictionUnitSyntax, ResidualCodingContext, ResidualCodingSyntax, SaoComponentSyntax,
    SaoSyntax, SliceSegmentDataContext, SliceSegmentDataSyntax, SliceSegmentLayerSyntax,
    TransformTreeContext, TransformTreeNode, TransformUnitSyntax,
};
pub use syntax::{
    ebsp_to_rbsp, parse_access_unit_delimiter_rbsp, parse_end_of_bitstream_rbsp,
    parse_end_of_sequence_rbsp, parse_filler_data_rbsp, parse_hrd_parameters,
    parse_nal_unit_syntax, parse_nal_unit_syntax_from_bytes,
    parse_rbsp_slice_segment_trailing_bits, parse_short_term_reference_picture_set,
    parse_slice_segment_header, parse_vui_parameters, AccessUnitDelimiterRbsp, BitReader,
    BitstreamRestriction, ColourMappingLeafSyntax, ColourMappingOctantSyntax,
    ColourMappingTableSyntax, CpbEntry, DeltaDltSyntax, FillerDataRbsp, HrdParameters,
    HrdSubLayerParameters, LongTermReferencePictureSetSyntax, NalUnitHeader, NalUnitSyntax,
    ParsedNalUnit, PcmSyntax, PictureParameterSetSyntax, Pps3dExtensionSyntax,
    PpsDeblockingFilterSyntax, PpsDepthDltSyntax, PpsExtensionSyntax, PpsMultilayerExtensionSyntax,
    PpsRangeExtensionSyntax, PpsReferenceLocationOffsetSyntax, PpsSccExtensionSyntax,
    PpsTileSyntax, ProfileInfo, ProfileTierLevel, ScalingListData, ScalingListMatrix, SeiMessage,
    SeiRbsp, SequenceParameterSetHeader, SequenceParameterSetSyntax, ShortTermReferencePictureSet,
    SliceLongTermReferencePicture, SlicePredictionWeightTable, SliceReferenceListModification,
    SliceReferencePictureSet, SliceSaoSyntax, SliceSegmentHeaderContext, SliceSegmentHeaderSyntax,
    SliceWeightList, Sps3dExtensionSyntax, SpsExtensionSyntax, SpsMultilayerExtensionSyntax,
    SpsRangeExtensionSyntax, SpsSccExtensionSyntax, SubLayerHrdParameters, SubLayerOrderingInfo,
    SubLayerProfileLevel, SyntaxDescriptor, SyntaxError, SyntaxValue, VideoParameterSetHeader,
    VideoParameterSetSyntax, VpsTimingSyntax, VuiParameters, VuiTimingInfo,
};
pub use tiles::TileLayout;
