#![allow(missing_docs)]

use h265rs::{
    apply_deblocking_edge, apply_sao_ctb, derive_chroma_intra_mode, derive_luma_intra_mode,
    derive_picture_order_count, fractional_luma_sample, intra_predict, inverse_transform,
    reconstruct_block, residual_bypass, Block, ChromaFormat, DeblockingParameters, EdgeDirection,
    IntraPredictionMode, IntraReferences, PictureDecodeContext, PictureFormat, SamplePlane,
    SaoBlock, SaoType,
};

fn format() -> PictureFormat {
    PictureFormat::new(16, 16, 8, 8, ChromaFormat::Yuv420, false).unwrap()
}

#[test]
fn picture_planes_clip_fractional_reference_coordinates() {
    let plane = SamplePlane::from_samples(4, 1, vec![0, 10, 20, 30]).unwrap();
    assert_eq!(plane.get_clipped(-5, 0), 0);
    assert_eq!(plane.get_clipped(50, 0), 30);
    assert_eq!(fractional_luma_sample(&plane, 1, 0, 0, 0, 8), 10);
}

#[test]
fn poc_wrap_and_irap_reset_follow_8_3_1() {
    assert_eq!(derive_picture_order_count(1, 14, 0, 16, false), 17);
    assert_eq!(derive_picture_order_count(14, 1, 16, 16, false), 14);
    assert_eq!(derive_picture_order_count(12, 8, 16, 16, true), 0);
}

#[test]
fn intra_mode_derivation_uses_mpm_and_remapping() {
    assert_eq!(derive_luma_intra_mode(0, 0, true, 2, 0), 26);
    assert_eq!(derive_luma_intra_mode(10, 10, true, 1, 0), 9);
    assert_eq!(derive_luma_intra_mode(0, 1, false, 0, 0), 2);
    assert_eq!(derive_chroma_intra_mode(0, 0, false), 34);
    assert_eq!(derive_chroma_intra_mode(1, 0, true), 26);
}

#[test]
fn planar_and_dc_prediction_are_reproducible() {
    let references = IntraReferences::new(10, vec![10; 9], vec![10; 9]);
    assert_eq!(
        intra_predict(&references, 4, 4, IntraPredictionMode::Dc, 8),
        vec![10; 16]
    );
    assert_eq!(
        intra_predict(&references, 4, 4, IntraPredictionMode::Planar, 8),
        vec![10; 16]
    );
}

#[test]
fn reconstruction_clips_prediction_plus_residual() {
    let result = reconstruct_block(&[250, 0, 0, 0], &[20, -20, 0, 0], Block::square(0, 0, 2), 8);
    assert_eq!(result, vec![255, 0, 0, 0]);
}

#[test]
fn inverse_transform_preserves_zero_block() {
    assert_eq!(inverse_transform(&[0; 16], 4, true, 8), vec![0; 16]);
}

#[test]
fn transform_bypass_accumulates_in_the_normative_direction() {
    let mut residual = vec![1, 2, 3, 4];
    residual_bypass(&mut residual, 2, true);
    assert_eq!(residual, vec![1, 3, 3, 7]);
}

#[test]
fn deblocking_changes_a_strong_step_without_touching_disabled_edges() {
    let mut plane = SamplePlane::new(16, 4, 0);
    for y in 0..4 {
        for x in 8..16 {
            plane.set(x, y, 255);
        }
    }
    let params = DeblockingParameters {
        boundary_strength: 2,
        bit_depth: 8,
        beta_offset_div2: 0,
        tc_offset_div2: 0,
        strong_filtering: true,
    };
    apply_deblocking_edge(&mut plane, 8, 0, EdgeDirection::Vertical, params);
    assert!(plane.get(7, 0).unwrap() > 0);
    assert!(plane.get(8, 0).unwrap() < 255);
    let before = plane.clone();
    apply_deblocking_edge(
        &mut plane,
        8,
        0,
        EdgeDirection::Vertical,
        DeblockingParameters {
            boundary_strength: 0,
            ..params
        },
    );
    assert_eq!(plane, before);
}

#[test]
fn sao_band_offsets_are_clipped_and_none_is_identity() {
    let plane = SamplePlane::new(4, 4, 16);
    let sao = SaoBlock {
        kind: SaoType::Band,
        band_position: 1,
        edge_class: 0,
        offsets: [0, 5, 0, 0, 0],
        bit_depth: 8,
    };
    let filtered = apply_sao_ctb(&plane, 0, 0, 4, 4, &sao);
    assert_eq!(filtered.get(0, 0), Some(21));
    let identity = SaoBlock {
        kind: SaoType::None,
        ..sao
    };
    assert_eq!(apply_sao_ctb(&plane, 0, 0, 4, 4, &identity), plane);
}

#[test]
fn format_is_used_by_chapter8_picture_storage() {
    let picture = h265rs::DecodedPicture::new(format());
    assert_eq!(picture.plane(0).unwrap().width(), 16);
    assert_eq!(picture.plane(1).unwrap().width(), 8);
}

#[test]
fn block_writes_copy_rows_and_clip_to_the_plane() {
    let mut plane = SamplePlane::new(4, 3, 0);
    plane.write_block(
        Block {
            x: 2,
            y: 1,
            width: 3,
            height: 2,
        },
        &[1, 2, 3, 4, 5, 6],
    );
    assert_eq!(plane.samples(), &[0, 0, 0, 0, 0, 0, 1, 2, 0, 0, 4, 5]);
}

#[test]
fn sao_plane_filters_each_ctb_without_changing_copy_semantics() {
    let format = PictureFormat::new(8, 4, 8, 8, ChromaFormat::Monochrome, false).unwrap();
    let context = PictureDecodeContext::new(format, 4);
    let plane = SamplePlane::from_samples(8, 4, (0..32).map(|index| 8 + (index % 8) * 8).collect())
        .unwrap();
    let parameters = SaoBlock {
        kind: SaoType::Band,
        band_position: 1,
        edge_class: 0,
        offsets: [0, 3, -2, 1, -1],
        bit_depth: 8,
    };
    assert_eq!(
        context.sao_plane(&plane, &parameters),
        apply_sao_ctb(&plane, 0, 0, 8, 4, &parameters)
    );
}
