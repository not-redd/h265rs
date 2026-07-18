#![allow(missing_docs)]

use h265rs::*;

fn geometry(width: u32, height: u32) -> PictureGeometry {
    let format = PictureFormat::new(width, height, 10, 10, ChromaFormat::Yuv420, false).unwrap();
    PictureGeometry::new(format, 4, 1).unwrap()
}

#[test]
fn table_6_1_dimensions_are_derived_correctly() {
    let format = PictureFormat::new(1920, 1080, 8, 8, ChromaFormat::Yuv420, false).unwrap();
    assert_eq!(
        format.component_dimension(0),
        Some(PlaneDimension {
            width: 1920,
            height: 1080
        })
    );
    assert_eq!(
        format.component_dimension(1),
        Some(PlaneDimension {
            width: 960,
            height: 540
        })
    );

    let format = PictureFormat::new(1920, 1080, 10, 8, ChromaFormat::Yuv422, false).unwrap();
    assert_eq!(
        format.component_dimension(1),
        Some(PlaneDimension {
            width: 960,
            height: 1080
        })
    );
    assert_eq!(format.prediction_block_count(), 5);
}

#[test]
fn picture_geometry_handles_incomplete_edge_ctbs() {
    let geometry = geometry(6, 5);
    assert_eq!(
        (geometry.width_in_ctbs(), geometry.height_in_ctbs()),
        (2, 2)
    );
    assert_eq!(
        geometry.ctb_bounds(0),
        Some(Block {
            x: 0,
            y: 0,
            width: 4,
            height: 4
        })
    );
    assert_eq!(
        geometry.ctb_bounds(1),
        Some(Block {
            x: 4,
            y: 0,
            width: 2,
            height: 4
        })
    );
    assert_eq!(
        geometry.ctb_bounds(3),
        Some(Block {
            x: 4,
            y: 4,
            width: 2,
            height: 1
        })
    );
}

#[test]
fn tiles_map_raster_and_tile_scan_addresses() {
    let layout = TileLayout::explicit(4, 2, vec![2, 2], vec![1, 1]).unwrap();
    assert_eq!(layout.raster_to_tile_scan(0), Some(0));
    assert_eq!(layout.raster_to_tile_scan(1), Some(1));
    assert_eq!(layout.raster_to_tile_scan(2), Some(2));
    assert_eq!(layout.tile_scan_to_raster(4), Some(4));
    assert_eq!(layout.tile_id(2), Some(1));

    let uniform = TileLayout::uniform(5, 1, 2, 1).unwrap();
    assert_eq!(uniform.column_widths(), &[2, 3]);
}

#[test]
fn quadtree_splits_in_specified_child_order() {
    let mut tree = QuadTree::new(Block::square(0, 0, 4));
    assert!(tree.split());
    assert_eq!(
        tree.leaves(),
        vec![
            Block::square(0, 0, 2),
            Block::square(2, 0, 2),
            Block::square(0, 2, 2),
            Block::square(2, 2, 2),
        ]
    );
    assert!(!tree.split());
}

#[test]
fn z_scan_is_inverse_and_matches_first_addresses() {
    let scan = z_scan_order(4).unwrap();
    assert_eq!(scan[..4], [(0, 0), (1, 0), (0, 1), (1, 1)]);
    let geometry = geometry(8, 4);
    let table = min_tb_address_table(&geometry);
    assert_eq!(table[0][0], 0);
    assert_eq!(table[0][1], 1);
    assert_eq!(table[1][0], 2);
    assert_eq!(table[0][4], 16);
}

#[test]
fn other_scan_orders_have_expected_sequences() {
    assert_eq!(
        horizontal_scan(2).unwrap(),
        vec![(0, 0), (1, 0), (0, 1), (1, 1)]
    );
    assert_eq!(
        vertical_scan(2).unwrap(),
        vec![(0, 0), (0, 1), (1, 0), (1, 1)]
    );
    assert_eq!(
        traverse_scan(2).unwrap(),
        vec![(0, 0), (1, 0), (1, 1), (0, 1)]
    );
    assert_eq!(
        up_right_diagonal_scan(2).unwrap(),
        vec![(0, 0), (0, 1), (1, 0), (1, 1)]
    );
    for scan in [
        up_right_diagonal_scan(4).unwrap(),
        horizontal_scan(4).unwrap(),
        vertical_scan(4).unwrap(),
        traverse_scan(4).unwrap(),
    ] {
        assert_eq!(scan.len(), 16);
        let mut sorted = scan.clone();
        sorted.sort_unstable();
        let mut expected = (0..4)
            .flat_map(|y| (0..4).map(move |x| (x, y)))
            .collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(sorted, expected);
    }
}

#[test]
fn availability_respects_picture_slice_and_tile_boundaries() {
    let geometry = geometry(8, 8);
    let tiles = TileLayout::explicit(2, 2, vec![2], vec![1, 1]).unwrap();
    let context = AvailabilityContext::new(geometry, tiles, vec![0, 0, 1, 1]).unwrap();
    assert!(context.z_scan_block_available((4, 0), (0, 0)));
    assert!(!context.z_scan_block_available((0, 4), (4, 0)));

    let tile_context = AvailabilityContext::new(
        geometry,
        TileLayout::explicit(2, 2, vec![1, 1], vec![2]).unwrap(),
        vec![0, 0, 0, 0],
    )
    .unwrap();
    assert!(!tile_context.z_scan_block_available((0, 4), (4, 0)));
    assert!(!context.z_scan_block_available((0, 0), (8, 0)));
}

#[test]
fn prediction_availability_applies_intra_and_partition_rules() {
    let geometry = geometry(8, 8);
    let tiles = TileLayout::uniform(2, 2, 1, 1).unwrap();
    let context = AvailabilityContext::new(geometry, tiles, vec![0, 0, 0, 0]).unwrap();
    let cb = Block::square(0, 0, 4);
    let pb = Block::square(0, 0, 2);
    assert!(!context.prediction_block_available(cb, pb, 1, (1, 2), PredictionMode::Inter));
    assert!(!context.prediction_block_available(cb, pb, 0, (0, 0), PredictionMode::Intra));
    let right_cb = Block::square(4, 0, 4);
    let right_pb = Block::square(4, 0, 2);
    assert!(context.prediction_block_available(
        right_cb,
        right_pb,
        0,
        (0, 0),
        PredictionMode::Inter
    ));
}

#[test]
fn byte_stream_round_trip_preserves_nal_payloads() {
    let first = [0x40, 0x01, 0xaa];
    let second = [0x02, 0xbb, 0xcc];
    let stream = nal_units_to_byte_stream(&[&first, &second]);
    assert_eq!(
        nal_units_from_byte_stream(&stream),
        vec![&first[..], &second[..]]
    );
}
