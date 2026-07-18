//! Clause 6 and Clause 7 building blocks for an H.265 decoder.
//!
//! The crate implements the structural, addressing, scan-order and basic syntax
//! processes described by ITU-T H.265 §§6-7. It does not implement prediction,
//! transforms, entropy coding, or the complete parameter-set/slice tables.

#![forbid(unsafe_code)]

mod availability;
mod bitstream;
mod error;
mod format;
mod geometry;
mod scan;
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
pub use syntax::{
    ebsp_to_rbsp, BitReader, NalUnitHeader, ParsedNalUnit, SyntaxDescriptor, SyntaxError,
    SyntaxValue,
};
pub use tiles::TileLayout;
