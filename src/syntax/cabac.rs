//! H.265 Clause 9.3 CABAC arithmetic decoding engine.

use super::{BitReader, SyntaxError};
use crate::{slice_data::PcmSampleSyntax, CabacReader};

/// One CABAC probability context variable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CabacContext {
    /// LPS probability-state index, 0..63.
    pub state_index: u8,
    /// Most probable symbol.
    pub value_mps: u8,
}

impl CabacContext {
    /// Creates a context with an explicitly specified state.
    pub const fn new(state_index: u8, value_mps: u8) -> Self {
        Self {
            state_index: if state_index > 63 { 63 } else { state_index },
            value_mps: value_mps & 1,
        }
    }

    /// Initializes a context from a Clause 9 `initValue` and slice QP.
    pub fn from_init_value(init_value: u8, slice_qp: i32) -> Self {
        let slope = i32::from(init_value >> 4) * 5 - 45;
        let offset = i32::from(init_value & 15) * 8 - 16;
        let pre_ctx_state = ((slope * slice_qp.clamp(0, 51)) >> 4) + offset;
        let pre_ctx_state = pre_ctx_state.clamp(1, 126);
        if pre_ctx_state <= 63 {
            Self::new((63 - pre_ctx_state) as u8, 0)
        } else {
            Self::new((pre_ctx_state - 64) as u8, 1)
        }
    }
}

/// CABAC context tables defined by H.265 Table 9-4.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CabacContextTable {
    /// SAO merge flags.
    SaoMerge = 0,
    /// SAO type index.
    SaoType,
    /// Coding-unit split flag.
    SplitCu,
    /// CU transquant bypass flag.
    CuTransquantBypass,
    /// CU skip flag.
    CuSkip,
    /// Prediction mode flag.
    PredMode,
    /// Partition mode.
    PartMode,
    /// Previous intra luma prediction flag.
    PrevIntraLumaPred,
    /// Intra chroma prediction mode.
    IntraChromaPred,
    /// Root coded-block flag.
    RqtRootCbf,
    /// Merge flag.
    Merge,
    /// Merge candidate index.
    MergeIdx,
    /// Inter prediction direction.
    InterPredIdc,
    /// Reference-picture index.
    RefIdx,
    /// Motion-vector predictor flag.
    MvpFlag,
    /// Transform split flag.
    SplitTransform,
    /// Luma coded-block flag.
    CbfLuma,
    /// Chroma coded-block flags.
    CbfChroma,
    /// Motion-vector absolute-value flags.
    AbsMvd,
    /// CU delta-QP absolute value.
    CuQpDeltaAbs,
    /// Transform skip flag.
    TransformSkip,
    /// Last significant coefficient X prefix.
    LastSigCoeffXPrefix,
    /// Last significant coefficient Y prefix.
    LastSigCoeffYPrefix,
    /// Coded sub-block flag.
    CodedSubBlock,
    /// Significant coefficient flag.
    SigCoeff,
    /// Coefficient absolute level greater-than-one flag.
    CoeffAbsLevelGreater1,
    /// Coefficient absolute level greater-than-two flag.
    CoeffAbsLevelGreater2,
    /// Explicit RDPCM flag.
    ExplicitRdpcm,
    /// Explicit RDPCM direction flag.
    ExplicitRdpcmDir,
    /// Chroma QP offset flag.
    ChromaQpOffsetFlag,
    /// Chroma QP offset index.
    ChromaQpOffsetIdx,
    /// Cross-component residual scale prefix.
    Log2ResScaleAbsPlus1,
    /// Cross-component residual scale sign.
    ResScaleSign,
    /// Palette mode flag.
    PaletteMode,
    /// Transform-unit residual ACT flag.
    TuResidualAct,
    /// Palette run prefix.
    PaletteRunPrefix,
    /// Palette copy-above flags.
    PaletteCopyAbove,
    /// Palette transpose flag.
    PaletteTranspose,
}

impl CabacContextTable {
    /// All context tables in the flat storage order used by [`CabacDecoder`].
    pub const ALL: [Self; 38] = [
        Self::SaoMerge,
        Self::SaoType,
        Self::SplitCu,
        Self::CuTransquantBypass,
        Self::CuSkip,
        Self::PredMode,
        Self::PartMode,
        Self::PrevIntraLumaPred,
        Self::IntraChromaPred,
        Self::RqtRootCbf,
        Self::Merge,
        Self::MergeIdx,
        Self::InterPredIdc,
        Self::RefIdx,
        Self::MvpFlag,
        Self::SplitTransform,
        Self::CbfLuma,
        Self::CbfChroma,
        Self::AbsMvd,
        Self::CuQpDeltaAbs,
        Self::TransformSkip,
        Self::LastSigCoeffXPrefix,
        Self::LastSigCoeffYPrefix,
        Self::CodedSubBlock,
        Self::SigCoeff,
        Self::CoeffAbsLevelGreater1,
        Self::CoeffAbsLevelGreater2,
        Self::ExplicitRdpcm,
        Self::ExplicitRdpcmDir,
        Self::ChromaQpOffsetFlag,
        Self::ChromaQpOffsetIdx,
        Self::Log2ResScaleAbsPlus1,
        Self::ResScaleSign,
        Self::PaletteMode,
        Self::TuResidualAct,
        Self::PaletteRunPrefix,
        Self::PaletteCopyAbove,
        Self::PaletteTranspose,
    ];

    /// Number of local context indices in this table.
    pub const fn size(self) -> usize {
        match self {
            Self::SaoMerge
            | Self::SaoType
            | Self::CuTransquantBypass
            | Self::IntraChromaPred
            | Self::PaletteMode
            | Self::TuResidualAct
            | Self::ChromaQpOffsetFlag
            | Self::ChromaQpOffsetIdx
            | Self::PaletteTranspose => 3,
            Self::SplitCu | Self::PartMode | Self::SplitTransform => 9,
            Self::CuSkip => 6,
            Self::PredMode | Self::RqtRootCbf | Self::Merge | Self::MergeIdx | Self::MvpFlag => 2,
            Self::PrevIntraLumaPred => 3,
            Self::InterPredIdc => 10,
            Self::RefIdx => 4,
            Self::CbfLuma => 6,
            Self::CbfChroma => 15,
            Self::AbsMvd => 4,
            Self::CuQpDeltaAbs => 6,
            Self::TransformSkip => 6,
            Self::LastSigCoeffXPrefix | Self::LastSigCoeffYPrefix => 54,
            Self::CodedSubBlock => 12,
            Self::SigCoeff => 132,
            Self::CoeffAbsLevelGreater1 => 72,
            Self::CoeffAbsLevelGreater2 => 18,
            Self::ExplicitRdpcm | Self::ExplicitRdpcmDir => 4,
            Self::Log2ResScaleAbsPlus1 => 24,
            Self::ResScaleSign => 6,
            Self::PaletteRunPrefix => 24,
            Self::PaletteCopyAbove => 3,
        }
    }

    /// Returns the flat context-vector index for a local `(ctxTable, ctxIdx)`.
    pub fn context_index(self, ctx_idx: usize) -> Result<usize, SyntaxError> {
        if ctx_idx >= self.size() {
            return Err(SyntaxError::InvalidSyntaxValue(
                "CABAC table context index is out of range",
            ));
        }
        let mut offset = 0;
        for table in Self::ALL {
            if table == self {
                return Ok(offset + ctx_idx);
            }
            offset += table.size();
        }
        Err(SyntaxError::InvalidSyntaxValue(
            "unknown CABAC context table",
        ))
    }

    /// Returns the `ctxIdxOffset` selected by Table 9-4 for an
    /// initialization type.
    pub fn context_offset(self, init_type: u8) -> Result<usize, SyntaxError> {
        let (_, offset) = self.init_values(init_type)?;
        Ok(offset)
    }

    /// Returns the standard init-value slice and its local starting index for
    /// the selected initialization type. Empty slices denote syntax elements
    /// that are not present for that initialization type.
    pub fn init_values(self, init_type: u8) -> Result<(&'static [u8], usize), SyntaxError> {
        if init_type > 2 {
            return Err(invalid_init_type());
        }
        let values = match self {
            Self::SaoMerge => (
                &SAO_MERGE_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::SaoType => (
                &SAO_TYPE_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::SplitCu => (
                &SPLIT_CU_INIT[init_type as usize * 3..init_type as usize * 3 + 3],
                init_type as usize * 3,
            ),
            Self::CuTransquantBypass => (
                &CU_TRANSQUANT_BYPASS_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::CuSkip => match init_type {
                0 => (&[][..], 0),
                1 => (&CU_SKIP_INIT[0..3], 0),
                2 => (&CU_SKIP_INIT[3..6], 3),
                _ => return Err(invalid_init_type()),
            },
            Self::PredMode => match init_type {
                0 => (&[][..], 0),
                1 => (&PRED_MODE_INIT[0..1], 0),
                2 => (&PRED_MODE_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::PartMode => match init_type {
                0 => (&PART_MODE_INIT[0..1], 0),
                1 => (&PART_MODE_INIT[1..5], 1),
                2 => (&PART_MODE_INIT[5..9], 5),
                _ => return Err(invalid_init_type()),
            },
            Self::PrevIntraLumaPred => (
                &PREV_INTRA_LUMA_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::IntraChromaPred => (
                &INTRA_CHROMA_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::RqtRootCbf => match init_type {
                0 => (&[][..], 0),
                1 => (&RQT_ROOT_CBF_INIT[0..1], 0),
                2 => (&RQT_ROOT_CBF_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::Merge => match init_type {
                0 => (&[][..], 0),
                1 => (&MERGE_INIT[0..1], 0),
                2 => (&MERGE_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::MergeIdx => match init_type {
                0 => (&[][..], 0),
                1 => (&MERGE_IDX_INIT[0..1], 0),
                2 => (&MERGE_IDX_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::InterPredIdc => match init_type {
                0 => (&[][..], 0),
                1 => (&INTER_PRED_IDC_INIT[0..5], 0),
                2 => (&INTER_PRED_IDC_INIT[5..10], 5),
                _ => return Err(invalid_init_type()),
            },
            Self::RefIdx => match init_type {
                0 => (&[][..], 0),
                1 => (&REF_IDX_INIT[0..2], 0),
                2 => (&REF_IDX_INIT[2..4], 2),
                _ => return Err(invalid_init_type()),
            },
            Self::MvpFlag => match init_type {
                0 => (&[][..], 0),
                1 => (&MVP_FLAG_INIT[0..1], 0),
                2 => (&MVP_FLAG_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::SplitTransform => (
                &SPLIT_TRANSFORM_INIT[init_type as usize * 3..init_type as usize * 3 + 3],
                init_type as usize * 3,
            ),
            Self::CbfLuma => (
                &CBF_LUMA_INIT[init_type as usize * 2..init_type as usize * 2 + 2],
                init_type as usize * 2,
            ),
            Self::CbfChroma => (
                &CBF_CHROMA_INIT[init_type as usize * 4..init_type as usize * 4 + 4],
                init_type as usize * 4,
            ),
            Self::AbsMvd => match init_type {
                0 => (&[][..], 0),
                1 => (&ABS_MVD_INIT[0..2], 0),
                2 => (&ABS_MVD_INIT[2..4], 2),
                _ => return Err(invalid_init_type()),
            },
            Self::CuQpDeltaAbs => (
                &CU_QP_DELTA_INIT[init_type as usize * 2..init_type as usize * 2 + 2],
                init_type as usize * 2,
            ),
            Self::TransformSkip => (
                &TRANSFORM_SKIP_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::LastSigCoeffXPrefix => (
                &LAST_SIG_X_INIT[init_type as usize * 18..init_type as usize * 18 + 18],
                init_type as usize * 18,
            ),
            Self::LastSigCoeffYPrefix => (
                &LAST_SIG_Y_INIT[init_type as usize * 18..init_type as usize * 18 + 18],
                init_type as usize * 18,
            ),
            Self::CodedSubBlock => (
                &CODED_SUB_BLOCK_INIT[init_type as usize * 4..init_type as usize * 4 + 4],
                init_type as usize * 4,
            ),
            Self::SigCoeff => (
                &SIG_COEFF_INIT[init_type as usize * 42..init_type as usize * 42 + 42],
                init_type as usize * 42,
            ),
            Self::CoeffAbsLevelGreater1 => (
                &COEFF_GREATER1_INIT[init_type as usize * 24..init_type as usize * 24 + 24],
                init_type as usize * 24,
            ),
            Self::CoeffAbsLevelGreater2 => (
                &COEFF_GREATER2_INIT[init_type as usize * 6..init_type as usize * 6 + 6],
                init_type as usize * 6,
            ),
            Self::ExplicitRdpcm => match init_type {
                0 => (&[][..], 0),
                1 => (&EXPLICIT_RDPCM_INIT[0..1], 0),
                2 => (&EXPLICIT_RDPCM_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::ExplicitRdpcmDir => match init_type {
                0 => (&[][..], 0),
                1 => (&EXPLICIT_RDPCM_DIR_INIT[0..1], 0),
                2 => (&EXPLICIT_RDPCM_DIR_INIT[1..2], 1),
                _ => return Err(invalid_init_type()),
            },
            Self::ChromaQpOffsetFlag => (
                &CHROMA_QP_OFFSET_FLAG_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::ChromaQpOffsetIdx => (
                &CHROMA_QP_OFFSET_IDX_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::Log2ResScaleAbsPlus1 => (
                &LOG2_RES_SCALE_INIT[init_type as usize * 8..init_type as usize * 8 + 8],
                init_type as usize * 8,
            ),
            Self::ResScaleSign => (
                &RES_SCALE_SIGN_INIT[init_type as usize * 2..init_type as usize * 2 + 2],
                init_type as usize * 2,
            ),
            Self::PaletteMode => (
                &PALETTE_MODE_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::TuResidualAct => (
                &TU_RESIDUAL_ACT_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::PaletteRunPrefix => (
                &PALETTE_RUN_PREFIX_INIT[init_type as usize * 8..init_type as usize * 8 + 8],
                init_type as usize * 8,
            ),
            Self::PaletteCopyAbove => (
                &PALETTE_COPY_ABOVE_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
            Self::PaletteTranspose => (
                &PALETTE_TRANSPOSE_INIT[init_type as usize..init_type as usize + 1],
                init_type as usize,
            ),
        };
        Ok(values)
    }

    /// Returns all `(local_start, init_values)` ranges for an initialization
    /// type. A few tables have a second single-context range in Table 9-4.
    pub fn init_ranges(self, init_type: u8) -> Result<Vec<(usize, &'static [u8])>, SyntaxError> {
        if init_type > 2 {
            return Err(invalid_init_type());
        }
        match self {
            Self::CbfChroma => Ok(vec![
                (
                    init_type as usize * 4,
                    &CBF_CHROMA_INIT[init_type as usize * 4..init_type as usize * 4 + 4],
                ),
                (
                    12 + init_type as usize,
                    &CBF_CHROMA_INIT[12 + init_type as usize..13 + init_type as usize],
                ),
            ]),
            Self::TransformSkip => Ok(vec![
                (
                    init_type as usize,
                    &TRANSFORM_SKIP_INIT[init_type as usize..init_type as usize + 1],
                ),
                (
                    3 + init_type as usize,
                    &TRANSFORM_SKIP_INIT[3 + init_type as usize..4 + init_type as usize],
                ),
            ]),
            Self::ExplicitRdpcm => match init_type {
                0 => Ok(Vec::new()),
                1 => Ok(vec![
                    (0, &EXPLICIT_RDPCM_INIT[0..1]),
                    (2, &EXPLICIT_RDPCM_INIT[2..3]),
                ]),
                2 => Ok(vec![
                    (1, &EXPLICIT_RDPCM_INIT[1..2]),
                    (3, &EXPLICIT_RDPCM_INIT[3..4]),
                ]),
                _ => Err(invalid_init_type()),
            },
            Self::ExplicitRdpcmDir => match init_type {
                0 => Ok(Vec::new()),
                1 => Ok(vec![
                    (0, &EXPLICIT_RDPCM_DIR_INIT[0..1]),
                    (2, &EXPLICIT_RDPCM_DIR_INIT[2..3]),
                ]),
                2 => Ok(vec![
                    (1, &EXPLICIT_RDPCM_DIR_INIT[1..2]),
                    (3, &EXPLICIT_RDPCM_DIR_INIT[3..4]),
                ]),
                _ => Err(invalid_init_type()),
            },
            Self::SigCoeff => Ok(vec![
                (
                    init_type as usize * 42,
                    &SIG_COEFF_INIT[init_type as usize * 42..init_type as usize * 42 + 42],
                ),
                (
                    126 + init_type as usize * 2,
                    &SIG_COEFF_INIT[126 + init_type as usize * 2..128 + init_type as usize * 2],
                ),
            ]),
            _ => {
                let (values, start) = self.init_values(init_type)?;
                Ok(vec![(start, values)])
            }
        }
    }
}

fn invalid_init_type() -> SyntaxError {
    SyntaxError::InvalidSyntaxValue("CABAC initialization type must be 0, 1, or 2")
}

/// Number of flat contexts in the standard Clause 9 tables.
pub const CABAC_CONTEXT_COUNT: usize = 531;

/// Derives `initType` from `slice_type` and `cabac_init_flag` as specified by
/// Equation (9-7). Slice types use the H.265 values: B=0, P=1, I=2.
pub const fn derive_cabac_init_type(
    slice_type: u8,
    cabac_init_flag: bool,
) -> Result<u8, SyntaxError> {
    match slice_type {
        2 => Ok(0),
        1 => Ok(if cabac_init_flag { 2 } else { 1 }),
        0 => Ok(if cabac_init_flag { 1 } else { 2 }),
        _ => Err(SyntaxError::InvalidSyntaxValue(
            "slice_type must be B, P, or I",
        )),
    }
}

const SAO_MERGE_INIT: [u8; 3] = [153, 153, 153];
const SAO_TYPE_INIT: [u8; 3] = [200, 185, 160];
const SPLIT_CU_INIT: [u8; 9] = [139, 141, 157, 107, 139, 126, 107, 139, 126];
const CU_TRANSQUANT_BYPASS_INIT: [u8; 3] = [154, 154, 154];
const CU_SKIP_INIT: [u8; 6] = [197, 185, 201, 197, 185, 201];
const PRED_MODE_INIT: [u8; 2] = [149, 134];
const PART_MODE_INIT: [u8; 9] = [184, 154, 139, 154, 154, 154, 139, 154, 154];
const PREV_INTRA_LUMA_INIT: [u8; 3] = [184, 154, 183];
const INTRA_CHROMA_INIT: [u8; 3] = [63, 152, 152];
const RQT_ROOT_CBF_INIT: [u8; 2] = [79, 79];
const MERGE_INIT: [u8; 2] = [110, 154];
const MERGE_IDX_INIT: [u8; 2] = [122, 137];
const INTER_PRED_IDC_INIT: [u8; 10] = [95, 79, 63, 31, 31, 95, 79, 63, 31, 31];
const REF_IDX_INIT: [u8; 4] = [153, 153, 153, 153];
const MVP_FLAG_INIT: [u8; 2] = [168, 168];
const SPLIT_TRANSFORM_INIT: [u8; 9] = [153, 138, 138, 124, 138, 94, 224, 167, 122];
const CBF_LUMA_INIT: [u8; 6] = [111, 141, 153, 111, 153, 111];
const CBF_CHROMA_INIT: [u8; 15] = [
    94, 138, 182, 154, 149, 107, 167, 154, 149, 92, 167, 154, 154, 154, 154,
];
const ABS_MVD_INIT: [u8; 4] = [140, 198, 169, 198];
const CU_QP_DELTA_INIT: [u8; 6] = [154, 154, 154, 154, 154, 154];
const TRANSFORM_SKIP_INIT: [u8; 6] = [139, 139, 139, 139, 139, 139];
const LAST_SIG_X_INIT: [u8; 54] = [
    110, 110, 124, 125, 140, 153, 125, 127, 140, 109, 111, 143, 127, 111, 79, 108, 123, 63, 125,
    110, 94, 110, 95, 79, 125, 111, 110, 78, 110, 111, 111, 95, 94, 108, 123, 108, 125, 110, 124,
    110, 95, 94, 125, 111, 111, 79, 125, 126, 111, 111, 79, 108, 123, 93,
];
const LAST_SIG_Y_INIT: [u8; 54] = LAST_SIG_X_INIT;
const CODED_SUB_BLOCK_INIT: [u8; 12] = [91, 171, 134, 141, 121, 140, 61, 154, 121, 140, 61, 154];
const SIG_COEFF_INIT: [u8; 132] = [
    111, 111, 125, 110, 110, 94, 124, 108, 124, 107, 125, 141, 179, 153, 125, 107, 125, 141, 179,
    153, 125, 107, 125, 141, 179, 153, 125, 140, 139, 182, 182, 152, 136, 152, 136, 153, 136, 139,
    111, 136, 139, 111, 155, 154, 139, 153, 139, 123, 123, 63, 153, 166, 183, 140, 136, 153, 154,
    166, 183, 140, 136, 153, 154, 166, 183, 140, 136, 153, 154, 170, 153, 123, 123, 107, 121, 107,
    121, 167, 151, 183, 140, 151, 183, 140, 170, 154, 139, 153, 139, 123, 123, 63, 124, 166, 183,
    140, 136, 153, 154, 166, 183, 140, 136, 153, 154, 166, 183, 140, 136, 153, 154, 170, 153, 138,
    138, 122, 121, 122, 121, 167, 151, 183, 140, 151, 183, 140, 141, 111, 140, 140, 140, 140,
];
const COEFF_GREATER1_INIT: [u8; 72] = [
    140, 92, 137, 138, 140, 152, 138, 139, 153, 74, 149, 92, 139, 107, 122, 152, 140, 179, 166,
    182, 140, 227, 122, 197, 154, 196, 196, 167, 154, 152, 167, 182, 182, 134, 149, 136, 153, 121,
    136, 137, 169, 194, 166, 167, 154, 167, 137, 182, 154, 196, 167, 167, 154, 152, 167, 182, 182,
    134, 149, 136, 153, 121, 136, 122, 169, 208, 166, 167, 154, 152, 167, 182,
];
const COEFF_GREATER2_INIT: [u8; 18] = [
    138, 153, 136, 167, 152, 152, 107, 167, 91, 122, 107, 167, 107, 167, 91, 107, 107, 167,
];
const EXPLICIT_RDPCM_INIT: [u8; 4] = [139, 139, 139, 139];
const EXPLICIT_RDPCM_DIR_INIT: [u8; 4] = [139, 139, 139, 139];
const CHROMA_QP_OFFSET_FLAG_INIT: [u8; 3] = [154, 154, 154];
const CHROMA_QP_OFFSET_IDX_INIT: [u8; 3] = [154, 154, 154];
const LOG2_RES_SCALE_INIT: [u8; 24] = [154; 24];
const RES_SCALE_SIGN_INIT: [u8; 6] = [154; 6];
const PALETTE_MODE_INIT: [u8; 3] = [154, 154, 154];
const TU_RESIDUAL_ACT_INIT: [u8; 3] = [154, 154, 154];
const PALETTE_RUN_PREFIX_INIT: [u8; 24] = [154; 24];
const PALETTE_COPY_ABOVE_INIT: [u8; 3] = [154, 154, 154];
const PALETTE_TRANSPOSE_INIT: [u8; 3] = [154, 154, 154];

/// Clause 9.3.4 CABAC arithmetic decoder.
#[derive(Clone, Debug)]
pub struct CabacDecoder<'a> {
    reader: BitReader<'a>,
    /// Arithmetic interval range.
    pub interval_range: u16,
    /// Arithmetic interval offset.
    pub offset: u32,
    contexts: Vec<CabacContext>,
    initial_contexts: Vec<CabacContext>,
    wpp_contexts: Option<Vec<CabacContext>>,
    init_type: Option<u8>,
    terminated: bool,
}

impl<'a> CabacDecoder<'a> {
    /// Initializes a decoder from the CABAC payload, consuming `ivlOffset`'s
    /// nine initialization bits as specified by §9.3.2.6.
    pub fn new(data: &'a [u8], context_count: usize) -> Result<Self, SyntaxError> {
        let mut reader = BitReader::new(data);
        let offset = reader.read_bits(9)? as u32;
        if offset >= 510 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "CABAC ivlOffset must not be 510 or 511",
            ));
        }
        Ok(Self {
            reader,
            interval_range: 510,
            offset,
            contexts: vec![CabacContext::new(0, 0); context_count.max(1)],
            initial_contexts: vec![CabacContext::new(0, 0); context_count.max(1)],
            wpp_contexts: None,
            init_type: None,
            terminated: false,
        })
    }

    /// Initializes a decoder using caller-provided context init values.
    pub fn with_init_values(
        data: &'a [u8],
        init_values: &[u8],
        slice_qp: i32,
    ) -> Result<Self, SyntaxError> {
        let mut decoder = Self::new(data, init_values.len())?;
        decoder.contexts = if init_values.is_empty() {
            vec![CabacContext::new(0, 0)]
        } else {
            init_values
                .iter()
                .map(|&value| CabacContext::from_init_value(value, slice_qp))
                .collect()
        };
        decoder.initial_contexts = decoder.contexts.clone();
        Ok(decoder)
    }

    /// Initializes all standard Clause 9.3 context tables for `init_type`.
    pub fn with_standard_contexts(
        data: &'a [u8],
        init_type: u8,
        slice_qp: i32,
    ) -> Result<Self, SyntaxError> {
        let mut decoder = Self::new(data, CABAC_CONTEXT_COUNT)?;
        decoder.init_type = Some(init_type);
        for table in CabacContextTable::ALL {
            for (start, values) in table.init_ranges(init_type)? {
                let base = table.context_index(0)?;
                for (index, &value) in values.iter().enumerate() {
                    decoder.contexts[base + start + index] =
                        CabacContext::from_init_value(value, slice_qp);
                }
            }
        }
        decoder.initial_contexts = decoder.contexts.clone();
        Ok(decoder)
    }

    /// Restores the context variables to the state produced by construction.
    /// Clause 9.3.2.1 invokes this context initialization at a new tile
    /// substream.
    pub fn reset_contexts_to_initial(&mut self) {
        self.contexts.clone_from(&self.initial_contexts);
        self.wpp_contexts = None;
    }

    /// Stores the adapted context variables after the second CTU in a WPP
    /// row, as required by Clause 9.3.2.4.
    pub fn store_wpp_contexts(&mut self) {
        self.wpp_contexts = Some(self.contexts.clone());
    }

    /// Restores the context variables stored for the preceding WPP row.
    /// Returns `false` when no preceding-row state is available.
    pub fn synchronize_wpp_contexts(&mut self) -> bool {
        if let Some(contexts) = &self.wpp_contexts {
            self.contexts.clone_from(contexts);
            true
        } else {
            false
        }
    }

    /// Replaces the decoder context state, useful for WPP/dependent-slice synchronization.
    pub fn set_contexts(&mut self, contexts: Vec<CabacContext>) {
        self.contexts = if contexts.is_empty() {
            vec![CabacContext::new(0, 0)]
        } else {
            contexts
        };
    }
    /// Returns a copy of all contexts for storage/synchronization.
    pub fn contexts(&self) -> &[CabacContext] {
        &self.contexts
    }
    /// Stores the context variables for WPP or dependent-slice restart.
    #[must_use]
    pub fn store_contexts(&self) -> Vec<CabacContext> {
        self.contexts.clone()
    }

    /// Restores context variables saved by [`Self::store_contexts`].
    pub fn synchronize_contexts(&mut self, stored: &[CabacContext]) -> Result<(), SyntaxError> {
        if stored.is_empty() {
            return Err(SyntaxError::InvalidSyntaxValue(
                "CABAC synchronization context state is empty",
            ));
        }
        if stored.len() != self.contexts.len() {
            return Err(SyntaxError::InvalidSyntaxValue(
                "CABAC synchronization context state has the wrong size",
            ));
        }
        self.contexts = stored.to_vec();
        Ok(())
    }
    /// Returns one mutable context for WPP/dependent-slice synchronization.
    pub fn context_mut(&mut self, index: usize) -> Option<&mut CabacContext> {
        self.contexts.get_mut(index)
    }
    /// Returns the current input bit position.
    pub const fn bit_position(&self) -> usize {
        self.reader.position()
    }
    /// Returns whether arithmetic termination has been decoded.
    pub const fn is_terminated(&self) -> bool {
        self.terminated
    }

    /// Reads PCM alignment bits and samples from the CABAC-owned payload,
    /// then invokes the arithmetic-engine initialization process again.
    pub fn read_pcm_sample_values(
        &mut self,
        luma_sample_count: usize,
        chroma_sample_count: usize,
        bit_depth_luma: usize,
        bit_depth_chroma: usize,
    ) -> Result<(Vec<u64>, Vec<u64>), SyntaxError> {
        while !self.reader.byte_aligned() {
            if self.reader.read_bits(1)? != 0 {
                return Err(SyntaxError::InvalidAlignmentZero);
            }
        }
        let mut luma_samples = Vec::with_capacity(luma_sample_count);
        for _ in 0..luma_sample_count {
            luma_samples.push(self.reader.read_bits(bit_depth_luma)?);
        }
        let mut chroma_samples = Vec::with_capacity(chroma_sample_count);
        for _ in 0..chroma_sample_count {
            chroma_samples.push(self.reader.read_bits(bit_depth_chroma)?);
        }
        self.reinitialize_after_pcm()?;
        Ok((luma_samples, chroma_samples))
    }

    /// Reinitializes `ivlCurrRange` and `ivlOffset` after PCM data while
    /// retaining the already-adapted context variables.
    pub fn reinitialize_after_pcm(&mut self) -> Result<(), SyntaxError> {
        self.initialize_arithmetic_engine()
    }

    /// Initializes the arithmetic engine for a new CABAC substream.
    pub fn initialize_arithmetic_engine(&mut self) -> Result<(), SyntaxError> {
        let offset = self.reader.read_bits(9)? as u32;
        if offset >= 510 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "CABAC ivlOffset must not be 510 or 511",
            ));
        }
        self.interval_range = 510;
        self.offset = offset;
        self.terminated = false;
        Ok(())
    }

    /// Decodes a context-coded bin using context index `ctx_idx`.
    pub fn decode_decision(&mut self, ctx_idx: usize) -> Result<u64, SyntaxError> {
        let context = self
            .contexts
            .get_mut(ctx_idx)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "CABAC context index is out of range",
            ))?;
        let q_range_idx = ((self.interval_range >> 6) & 3) as usize;
        let lps_range = RANGE_TAB_LPS[context.state_index as usize][q_range_idx] as u16;
        self.interval_range -= lps_range;
        let bin = if self.offset >= u32::from(self.interval_range) {
            self.offset -= u32::from(self.interval_range);
            self.interval_range = lps_range;
            let lps = 1 - context.value_mps;
            if context.state_index == 0 {
                context.value_mps ^= 1;
            }
            context.state_index = TRANS_IDX_LPS[context.state_index as usize];
            lps
        } else {
            context.state_index = TRANS_IDX_MPS[context.state_index as usize];
            context.value_mps
        };
        self.renormalize()?;
        Ok(u64::from(bin))
    }

    /// Decodes a bin from a named Clause 9 context table.
    pub fn decode_context(
        &mut self,
        table: CabacContextTable,
        ctx_idx: usize,
    ) -> Result<u64, SyntaxError> {
        self.decode_decision(table.context_index(ctx_idx)?)
    }

    /// Decodes a bin using the context increment from Table 9-48. The
    /// initialization-type offset from Table 9-4 is applied automatically
    /// for a decoder created with [`Self::with_standard_contexts`].
    pub fn decode_syntax_context(
        &mut self,
        table: CabacContextTable,
        ctx_inc: usize,
    ) -> Result<u64, SyntaxError> {
        self.decode_decision(self.syntax_context_index(table, ctx_inc)?)
    }

    /// Returns the flat context-vector index selected by a syntax context
    /// increment.
    pub fn syntax_context_index(
        &self,
        table: CabacContextTable,
        ctx_inc: usize,
    ) -> Result<usize, SyntaxError> {
        if table == CabacContextTable::CbfChroma && ctx_inc >= 12 {
            return table.context_index(ctx_inc);
        }
        if table == CabacContextTable::SigCoeff && ctx_inc >= 42 {
            let special_index = self.init_type.map_or(ctx_inc, |init_type| {
                126 + usize::from(init_type) * 2 + ctx_inc - 42
            });
            return table.context_index(special_index);
        }
        let offset = self
            .init_type
            .map_or(Ok(0), |init_type| table.context_offset(init_type))?;
        table.context_index(offset + ctx_inc)
    }

    /// Decodes a bypass bin (§9.3.4.3.4).
    pub fn decode_bypass(&mut self) -> Result<u64, SyntaxError> {
        self.offset = (self.offset << 1) | self.reader.read_bits(1)? as u32;
        if self.offset >= u32::from(self.interval_range) {
            self.offset -= u32::from(self.interval_range);
            if self.offset >= u32::from(self.interval_range) {
                return Err(SyntaxError::InvalidSyntaxValue(
                    "CABAC offset is outside the arithmetic interval after bypass decoding",
                ));
            }
            Ok(1)
        } else {
            Ok(0)
        }
    }

    /// Decodes a termination bin (§9.3.4.3.5).
    pub fn decode_terminate(&mut self) -> Result<u64, SyntaxError> {
        self.interval_range -= 2;
        if self.offset >= u32::from(self.interval_range) {
            self.terminated = true;
            Ok(1)
        } else {
            self.renormalize()?;
            Ok(0)
        }
    }

    /// Reads `count` bypass-coded bits, MSB first.
    pub fn read_bypass_bits(&mut self, count: usize) -> Result<u64, SyntaxError> {
        if count > 64 {
            return Err(SyntaxError::InvalidBitCount(count));
        }
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | self.decode_bypass()?;
        }
        Ok(value)
    }

    /// Reads a truncated-binary value using bypass bins.
    pub fn read_truncated_binary_bypass(&mut self, c_max: u64) -> Result<u64, SyntaxError> {
        let n = c_max.checked_add(1).ok_or(SyntaxError::InvalidSyntaxValue(
            "truncated-binary range is too large",
        ))?;
        let k = 63 - n.leading_zeros();
        let u = (1_u64 << (k + 1)) - n;
        let mut value = self.read_bypass_bits(k as usize)?;
        if value < u {
            return Ok(value);
        }
        value = (value << 1) | self.decode_bypass()?;
        let value = value - u;
        (value <= c_max)
            .then_some(value)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "truncated-binary value exceeds cMax",
            ))
    }

    /// Sets the arithmetic range to 256 before aligned bypass decoding.
    pub const fn align_bypass(&mut self) {
        self.interval_range = 256;
    }

    fn renormalize(&mut self) -> Result<(), SyntaxError> {
        while self.interval_range < 256 {
            self.interval_range <<= 1;
            self.offset = (self.offset << 1) | self.reader.read_bits(1)? as u32;
        }
        if self.offset >= u32::from(self.interval_range) {
            return Err(SyntaxError::InvalidSyntaxValue(
                "CABAC offset is outside the arithmetic interval",
            ));
        }
        Ok(())
    }

    fn read_rbsp_trailing_bits(&mut self) -> Result<usize, SyntaxError> {
        if !self.terminated && self.decode_terminate()? != 1 {
            return Err(SyntaxError::InvalidAlignmentBit);
        }
        if !self.reader.byte_aligned() {
            while !self.reader.byte_aligned() {
                if self.reader.read_bits(1)? != 0 {
                    return Err(SyntaxError::InvalidAlignmentZero);
                }
            }
        }
        let mut cabac_zero_word_count = 0;
        while self.reader.bits_remaining() >= 16 {
            if self.reader.read_bits(16)? != 0 {
                return Err(SyntaxError::InvalidSyntaxValue(
                    "cabac_zero_word must equal 0x0000",
                ));
            }
            cabac_zero_word_count += 1;
        }
        if self.reader.bits_remaining() != 0 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "trailing CABAC data is not a complete cabac_zero_word",
            ));
        }
        Ok(cabac_zero_word_count)
    }
}

impl CabacReader for CabacDecoder<'_> {
    fn read_ae(&mut self) -> Result<u64, SyntaxError> {
        self.decode_decision(0)
    }
    fn read_ae_context(&mut self, ctx_idx: usize) -> Result<u64, SyntaxError> {
        self.decode_decision(ctx_idx)
    }
    fn read_ae_named(
        &mut self,
        table: CabacContextTable,
        ctx_idx: usize,
    ) -> Result<u64, SyntaxError> {
        self.decode_syntax_context(table, ctx_idx)
    }
    fn read_bits(&mut self, count: usize) -> Result<u64, SyntaxError> {
        self.read_bypass_bits(count)
    }
    fn read_pcm_samples(
        &mut self,
        luma_sample_count: usize,
        chroma_sample_count: usize,
        bit_depth_luma: usize,
        bit_depth_chroma: usize,
    ) -> Result<PcmSampleSyntax, SyntaxError> {
        let (luma_samples, chroma_samples) = self.read_pcm_sample_values(
            luma_sample_count,
            chroma_sample_count,
            bit_depth_luma,
            bit_depth_chroma,
        )?;
        Ok(PcmSampleSyntax {
            luma_samples,
            chroma_samples,
        })
    }
    fn read_bypass_bin(&mut self) -> Result<u64, SyntaxError> {
        self.decode_bypass()
    }
    fn read_bypass_bits(&mut self, count: usize) -> Result<u64, SyntaxError> {
        CabacDecoder::read_bypass_bits(self, count)
    }
    fn read_truncated_rice(&mut self, c_max: u64, rice_parameter: u8) -> Result<u64, SyntaxError> {
        if rice_parameter >= 64 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "truncated-Rice parameter is too large",
            ));
        }
        let rice = u32::from(rice_parameter);
        let max_prefix = c_max >> rice;
        let mut prefix = 0_u64;
        while prefix < max_prefix && self.decode_bypass()? != 0 {
            prefix += 1;
        }
        if prefix == max_prefix {
            return Ok(c_max);
        }
        let suffix = self.read_bypass_bits(rice_parameter as usize)?;
        let value = (prefix << rice) | suffix;
        (value <= c_max)
            .then_some(value)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "truncated-Rice value exceeds cMax",
            ))
    }
    fn read_truncated_rice_context(
        &mut self,
        table: CabacContextTable,
        ctx_idx: usize,
        context_bins: usize,
        c_max: u64,
        rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        if rice_parameter >= 64 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "truncated-Rice parameter is too large",
            ));
        }
        let rice = u32::from(rice_parameter);
        let max_prefix = c_max >> rice;
        let mut prefix = 0_u64;
        while prefix < max_prefix {
            let bin = if prefix as usize >= context_bins {
                self.decode_bypass()?
            } else {
                self.decode_syntax_context(table, ctx_idx + prefix as usize)?
            };
            if bin == 0 {
                break;
            }
            prefix += 1;
        }
        if prefix == max_prefix {
            return Ok(c_max);
        }
        let suffix = self.read_bypass_bits(rice_parameter as usize)?;
        let value = (prefix << rice) | suffix;
        (value <= c_max)
            .then_some(value)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "context truncated-Rice value exceeds cMax",
            ))
    }
    fn read_truncated_rice_repeated_context(
        &mut self,
        table: CabacContextTable,
        ctx_inc: usize,
        context_bins: usize,
        c_max: u64,
        rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        if rice_parameter >= 64 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "truncated-Rice parameter is too large",
            ));
        }
        let rice = u32::from(rice_parameter);
        let max_prefix = c_max >> rice;
        let mut prefix = 0_u64;
        while prefix < max_prefix {
            let bin = if prefix as usize >= context_bins {
                self.decode_bypass()?
            } else {
                self.decode_syntax_context(table, ctx_inc)?
            };
            if bin == 0 {
                break;
            }
            prefix += 1;
        }
        if prefix == max_prefix {
            return Ok(c_max);
        }
        let suffix = self.read_bypass_bits(rice_parameter as usize)?;
        let value = (prefix << rice) | suffix;
        (value <= c_max)
            .then_some(value)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "repeated-context truncated-Rice value exceeds cMax",
            ))
    }
    fn read_fixed_bypass(&mut self, c_max: u64) -> Result<u64, SyntaxError> {
        let width = if c_max == 0 {
            0
        } else {
            (64 - c_max.leading_zeros()) as usize
        };
        let value = self.read_bypass_bits(width)?;
        (value <= c_max)
            .then_some(value)
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "fixed-length value exceeds cMax",
            ))
    }
    fn read_exp_golomb(&mut self, order: u8) -> Result<u64, SyntaxError> {
        let mut order = u32::from(order);
        let mut value = 0_u64;
        loop {
            let threshold = 1_u64
                .checked_shl(order)
                .ok_or(SyntaxError::ExpGolombOverflow)?;
            if self.decode_bypass()? != 0 {
                value = value
                    .checked_add(threshold)
                    .ok_or(SyntaxError::ExpGolombOverflow)?;
                order = order.checked_add(1).ok_or(SyntaxError::ExpGolombOverflow)?;
            } else {
                let suffix = self.read_bypass_bits(order as usize)?;
                return value
                    .checked_add(suffix)
                    .ok_or(SyntaxError::ExpGolombOverflow);
            }
        }
    }
    fn read_limited_exp_golomb(
        &mut self,
        rice_parameter: u8,
        log2_transform_range: u8,
    ) -> Result<u64, SyntaxError> {
        if rice_parameter >= 32 || !(1..=28).contains(&log2_transform_range) {
            return Err(SyntaxError::InvalidSyntaxValue(
                "limited EGk parameters are out of range",
            ));
        }
        let max_prefix_extension_length = 28 - log2_transform_range;
        let mut prefix_extension_length = 0_u8;
        while prefix_extension_length < max_prefix_extension_length && self.decode_bypass()? != 0 {
            prefix_extension_length += 1;
        }
        let escape_length = if prefix_extension_length == max_prefix_extension_length {
            log2_transform_range
        } else {
            prefix_extension_length + rice_parameter
        };
        let suffix = self.read_bypass_bits(escape_length as usize)?;
        let base = ((1_u64 << prefix_extension_length) - 1) << rice_parameter;
        let value = base
            .checked_add(suffix)
            .ok_or(SyntaxError::ExpGolombOverflow)?;
        if prefix_extension_length < max_prefix_extension_length
            && (value >> rice_parameter) > ((2_u64 << prefix_extension_length) - 2)
        {
            return Err(SyntaxError::InvalidSyntaxValue(
                "limited EGk prefix and suffix are inconsistent",
            ));
        }
        Ok(value)
    }
    fn read_palette_value(
        &mut self,
        bit_depth: usize,
        cu_transquant_bypass_flag: bool,
    ) -> Result<u64, SyntaxError> {
        if cu_transquant_bypass_flag {
            let c_max = (1_u64)
                .checked_shl(u32::try_from(bit_depth).map_err(|_| {
                    SyntaxError::InvalidSyntaxValue("palette bit depth is too large")
                })?)
                .and_then(|value| value.checked_sub(1))
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "palette bit depth is too large",
                ))?;
            self.read_fixed_bypass(c_max)
        } else {
            self.read_exp_golomb(3)
        }
    }
    fn read_palette_num_indices(&mut self, max_palette_index: u64) -> Result<u64, SyntaxError> {
        let rice_parameter = 3_u64
            .checked_add(
                max_palette_index
                    .checked_add(1)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "palette index maximum overflows",
                    ))?
                    >> 3,
            )
            .and_then(|value| u8::try_from(value).ok())
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "palette index Rice parameter is too large",
            ))?;
        let c_max =
            4_u64
                .checked_shl(u32::from(rice_parameter))
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "palette index Rice range is too large",
                ))?;
        let prefix = self.read_truncated_rice(c_max, rice_parameter)?;
        if prefix == c_max {
            self.read_exp_golomb(rice_parameter.saturating_add(1))?
                .checked_add(c_max)
                .ok_or(SyntaxError::ExpGolombOverflow)
        } else {
            Ok(prefix)
        }
    }
    fn read_palette_index(
        &mut self,
        max_palette_index: u64,
        _first: bool,
    ) -> Result<u64, SyntaxError> {
        self.read_truncated_binary_bypass(max_palette_index)
    }
    fn read_palette_run_prefix(
        &mut self,
        palette_max_run_minus1: u64,
        palette_index: u64,
        copy_above: bool,
    ) -> Result<u64, SyntaxError> {
        if palette_max_run_minus1 == 0 {
            return Ok(0);
        }
        let c_max = 64 - palette_max_run_minus1.leading_zeros();
        let mut prefix = 0_u64;
        while prefix < u64::from(c_max) {
            let bin_idx = prefix as usize;
            let ctx_inc = if bin_idx == 0 && !copy_above {
                if palette_index < 1 {
                    0
                } else if palette_index < 3 {
                    1
                } else {
                    2
                }
            } else if copy_above {
                match bin_idx {
                    0 => 5,
                    1 | 2 => 6,
                    3 | 4 => 7,
                    _ => {
                        if self.decode_bypass()? == 0 {
                            return Ok(prefix);
                        }
                        prefix += 1;
                        continue;
                    }
                }
            } else {
                match bin_idx {
                    1 | 2 => 3,
                    3 | 4 => 4,
                    _ => {
                        if self.decode_bypass()? == 0 {
                            return Ok(prefix);
                        }
                        prefix += 1;
                        continue;
                    }
                }
            };
            if self.decode_syntax_context(CabacContextTable::PaletteRunPrefix, ctx_inc)? == 0 {
                return Ok(prefix);
            }
            prefix += 1;
        }
        Ok(prefix)
    }
    fn read_palette_run_suffix(
        &mut self,
        palette_max_run_minus1: u64,
        palette_run_prefix: u64,
    ) -> Result<u64, SyntaxError> {
        let prefix_offset =
            1_u64
                .checked_shl(u32::try_from(palette_run_prefix - 1).map_err(|_| {
                    SyntaxError::InvalidSyntaxValue("palette run prefix is too large")
                })?)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "palette run prefix is too large",
                ))?;
        let c_max = if prefix_offset * 2 > palette_max_run_minus1 {
            palette_max_run_minus1 - prefix_offset
        } else {
            prefix_offset - 1
        };
        self.read_truncated_binary_bypass(c_max)
    }
    fn read_part_mode(
        &mut self,
        is_intra: bool,
        at_minimum_size: bool,
    ) -> Result<u64, SyntaxError> {
        self.read_part_mode_with_amp(is_intra, at_minimum_size, false)
    }
    fn read_part_mode_with_amp(
        &mut self,
        is_intra: bool,
        at_minimum_size: bool,
        amp_enabled: bool,
    ) -> Result<u64, SyntaxError> {
        let first = self.decode_syntax_context(CabacContextTable::PartMode, 0)?;
        if is_intra {
            return Ok(if first != 0 { 0 } else { 1 });
        }
        if first != 0 {
            return Ok(0);
        }
        let second = self.decode_syntax_context(CabacContextTable::PartMode, 1)?;
        if second != 0 {
            if at_minimum_size || !amp_enabled {
                return Ok(1);
            }
            let third = self.decode_bypass()?;
            if third != 0 {
                return Ok(1);
            }
            return Ok(if self.decode_bypass()? == 0 { 4 } else { 5 });
        }
        if at_minimum_size {
            return Ok(2);
        }
        let third = self.decode_syntax_context(CabacContextTable::PartMode, 2)?;
        if third != 0 {
            return Ok(2);
        }
        if !amp_enabled {
            return Ok(3);
        }
        let fourth = self.decode_bypass()?;
        if fourth != 0 {
            return Ok(3);
        }
        Ok(if self.decode_bypass()? == 0 { 6 } else { 7 })
    }
    fn read_inter_pred_idc(
        &mut self,
        n_pb_w: u64,
        n_pb_h: u64,
        ct_depth: usize,
    ) -> Result<u64, SyntaxError> {
        let first_ctx = if n_pb_w + n_pb_h != 12 { ct_depth } else { 4 };
        let first = self.decode_syntax_context(CabacContextTable::InterPredIdc, first_ctx)?;
        if n_pb_w + n_pb_h == 12 {
            return Ok(if first == 0 { 0 } else { 2 });
        }
        if first != 0 {
            return Ok(2);
        }
        let second = self.decode_syntax_context(CabacContextTable::InterPredIdc, 4)?;
        Ok(if second == 0 { 0 } else { 1 })
    }
    fn read_cu_qp_delta_abs(&mut self) -> Result<u64, SyntaxError> {
        let mut prefix = 0_u64;
        while prefix < 5 {
            let bin = if prefix == 0 {
                self.decode_syntax_context(CabacContextTable::CuQpDeltaAbs, 0)?
            } else {
                self.decode_syntax_context(CabacContextTable::CuQpDeltaAbs, 1)?
            };
            if bin == 0 {
                return Ok(prefix);
            }
            prefix += 1;
        }
        self.read_exp_golomb(0)?
            .checked_add(5)
            .ok_or(SyntaxError::ExpGolombOverflow)
    }
    fn read_coeff_abs_level_remaining(&mut self, _base_level: u64) -> Result<u64, SyntaxError> {
        let prefix = self.read_truncated_rice(4, 0)?;
        if prefix == 4 {
            self.read_exp_golomb(1)?
                .checked_add(4)
                .ok_or(SyntaxError::ExpGolombOverflow)
        } else {
            Ok(prefix)
        }
    }
    fn read_coeff_abs_level_remaining_with_parameters(
        &mut self,
        _base_level: u64,
        rice_parameter: u8,
    ) -> Result<u64, SyntaxError> {
        if rice_parameter > 4 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "coefficient Rice parameter is too large",
            ));
        }
        let c_max = 4_u64
            .checked_shl(u32::from(rice_parameter))
            .ok_or(SyntaxError::ExpGolombOverflow)?;
        let prefix = self.read_truncated_rice(c_max, rice_parameter)?;
        if prefix == c_max {
            self.read_exp_golomb(rice_parameter.saturating_add(1))?
                .checked_add(c_max)
                .ok_or(SyntaxError::ExpGolombOverflow)
        } else {
            Ok(prefix)
        }
    }
    fn read_coeff_abs_level_remaining_with_options(
        &mut self,
        _base_level: u64,
        rice_parameter: u8,
        extended_precision: bool,
        log2_transform_range: u8,
    ) -> Result<u64, SyntaxError> {
        if rice_parameter >= 32 {
            return Err(SyntaxError::InvalidSyntaxValue(
                "coefficient Rice parameter is too large",
            ));
        }
        let c_max = 4_u64
            .checked_shl(u32::from(rice_parameter))
            .ok_or(SyntaxError::ExpGolombOverflow)?;
        let prefix = self.read_truncated_rice(c_max, rice_parameter)?;
        if prefix != c_max {
            return Ok(prefix);
        }
        let suffix = if extended_precision {
            self.read_limited_exp_golomb(rice_parameter.saturating_add(1), log2_transform_range)?
        } else {
            self.read_exp_golomb(rice_parameter.saturating_add(1))?
        };
        c_max
            .checked_add(suffix)
            .ok_or(SyntaxError::ExpGolombOverflow)
    }
    fn cabac_bypass_alignment(&mut self) -> Result<(), SyntaxError> {
        self.align_bypass();
        Ok(())
    }
    fn byte_alignment(&mut self) -> Result<(), SyntaxError> {
        let terminated_by_alignment = self.terminated;
        if !terminated_by_alignment && self.reader.read_bits(1)? != 1 {
            return Err(SyntaxError::InvalidAlignmentBit);
        }
        while !self.reader.byte_aligned() {
            if self.reader.read_bits(1)? != 0 {
                return Err(SyntaxError::InvalidAlignmentZero);
            }
        }
        if terminated_by_alignment {
            self.terminated = false;
        }
        Ok(())
    }
    fn initialize_arithmetic_engine(&mut self) -> Result<(), SyntaxError> {
        CabacDecoder::initialize_arithmetic_engine(self)
    }
    fn reset_contexts_to_initial(&mut self) {
        CabacDecoder::reset_contexts_to_initial(self);
    }
    fn store_wpp_contexts(&mut self) {
        CabacDecoder::store_wpp_contexts(self);
    }
    fn synchronize_wpp_contexts(&mut self) -> bool {
        CabacDecoder::synchronize_wpp_contexts(self)
    }
    fn read_terminate(&mut self) -> Result<u64, SyntaxError> {
        self.decode_terminate()
    }
    fn rbsp_slice_segment_trailing_bits(&mut self) -> Result<usize, SyntaxError> {
        self.read_rbsp_trailing_bits()
    }
}

const RANGE_TAB_LPS: [[u8; 4]; 64] = [
    [128, 176, 208, 240],
    [128, 167, 197, 227],
    [128, 158, 187, 216],
    [123, 150, 178, 205],
    [116, 142, 169, 195],
    [111, 135, 160, 185],
    [105, 128, 152, 175],
    [100, 122, 144, 166],
    [95, 116, 137, 158],
    [90, 110, 130, 150],
    [85, 104, 123, 142],
    [81, 99, 117, 135],
    [77, 94, 111, 128],
    [73, 89, 105, 122],
    [69, 85, 100, 116],
    [66, 80, 95, 110],
    [62, 76, 90, 104],
    [59, 72, 86, 99],
    [56, 69, 81, 94],
    [53, 65, 77, 89],
    [51, 62, 73, 85],
    [48, 59, 69, 80],
    [46, 56, 66, 76],
    [43, 53, 63, 72],
    [41, 50, 59, 69],
    [39, 48, 56, 65],
    [37, 45, 54, 62],
    [35, 43, 51, 59],
    [33, 41, 48, 56],
    [32, 39, 46, 53],
    [30, 37, 43, 50],
    [29, 35, 41, 48],
    [32, 27, 33, 39],
    [33, 26, 31, 37],
    [34, 24, 30, 35],
    [35, 23, 28, 33],
    [36, 22, 27, 32],
    [37, 21, 26, 30],
    [38, 20, 24, 29],
    [39, 19, 23, 27],
    [40, 18, 22, 26],
    [41, 17, 21, 25],
    [42, 16, 20, 23],
    [43, 15, 19, 22],
    [44, 14, 18, 21],
    [45, 14, 17, 20],
    [46, 13, 16, 19],
    [47, 12, 15, 18],
    [48, 12, 14, 17],
    [49, 11, 14, 16],
    [50, 11, 13, 15],
    [51, 10, 12, 15],
    [52, 10, 12, 14],
    [53, 9, 11, 13],
    [54, 9, 11, 12],
    [55, 8, 10, 12],
    [56, 8, 9, 11],
    [57, 7, 9, 11],
    [58, 7, 9, 10],
    [59, 7, 8, 10],
    [60, 6, 8, 9],
    [61, 6, 7, 9],
    [62, 6, 7, 8],
    [63, 2, 2, 2],
];

const TRANS_IDX_LPS: [u8; 64] = [
    0, 0, 1, 2, 2, 4, 4, 5, 6, 7, 8, 9, 9, 11, 11, 12, 13, 13, 15, 15, 16, 16, 18, 18, 19, 19, 21,
    21, 22, 22, 23, 24, 24, 25, 26, 26, 27, 27, 28, 29, 29, 30, 30, 30, 31, 32, 32, 33, 33, 33, 34,
    34, 35, 35, 35, 36, 36, 36, 37, 37, 37, 38, 38, 63,
];
const TRANS_IDX_MPS: [u8; 64] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 62, 63,
];
