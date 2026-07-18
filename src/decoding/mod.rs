//! H.265 Clause 8 decoding processes.
//!
//! The module is deliberately data-oriented.  Syntax parsing remains in
//! [`crate::syntax`], while these functions operate on decoded syntax values,
//! sample planes and decoder state.  This makes the processes useful with a
//! software CABAC implementation as well as with conformance-test drivers.

mod filters;
mod inter;
mod intra;
mod picture;
mod process;
mod reference;
mod transform;

pub use filters::{
    apply_deblocking_edge, apply_sao_ctb, DeblockingParameters, EdgeDirection, SaoBlock, SaoType,
};
pub use inter::{
    default_weighted_prediction, derive_chroma_motion_vector, derive_merge_candidates,
    derive_motion_vector, fractional_chroma_sample, fractional_luma_sample, inter_predict,
    weighted_prediction, MotionVector, MotionVectorPrediction, PredictionLists, WeightParameters,
};
pub use intra::{
    derive_chroma_intra_mode, derive_luma_intra_mode, intra_predict, IntraPredictionMode,
    IntraReferences,
};
pub use picture::{DecodedPicture, SamplePlane};
pub use process::{decode_palette_block, DecoderState, PictureDecodeContext};
pub use reference::{
    build_reference_picture_lists, derive_collocated_picture_and_no_backward_prediction_flag,
    derive_picture_order_count, derive_reference_set, derive_reference_set_from_sets,
    diff_picture_order_count, generate_unavailable_picture, slice_short_term_reference_set,
    DecodedPictureBuffer, PictureMarking, ReferencePictureLists, ReferenceSet,
};
pub use transform::{
    adaptive_colour_transform, cross_component_residual, inverse_transform, reconstruct_block,
    residual_bypass, scale_transform_coefficients, TransformParameters,
};

/// Clips an intermediate sample to the legal range for a component.
pub fn clip_sample(value: i64, bit_depth: u8) -> i32 {
    let max = (1_i64 << bit_depth) - 1;
    if value < 0 {
        0
    } else if value > max {
        max as i32
    } else {
        value as i32
    }
}

/// H.265's rounded right shift for non-negative and signed intermediates.
pub const fn rounded_shift(value: i64, shift: u32) -> i64 {
    if shift == 0 {
        value
    } else {
        (value + (1_i64 << (shift - 1))) >> shift
    }
}
