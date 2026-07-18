use super::{BitReader, SyntaxError};

/// One matrix entry from `scaling_list_data()` in §7.3.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalingListMatrix {
    /// Matrix size identifier, 0 through 3.
    pub size_id: u8,
    /// Matrix identifier within the size group.
    pub matrix_id: u8,
    /// `scaling_list_pred_mode_flag`.
    pub pred_mode_flag: bool,
    /// `scaling_list_pred_matrix_id_delta`, when prediction mode is used.
    pub pred_matrix_id_delta: Option<u64>,
    /// `scaling_list_dc_coef_minus8`, for size IDs greater than 1.
    pub dc_coef_minus8: Option<i64>,
    /// Parsed `scaling_list_delta_coef` values.
    pub delta_coefficients: Vec<i64>,
    /// Derived scaling coefficients, when prediction mode is not used.
    pub coefficients: Vec<u8>,
}

/// Parsed `scaling_list_data()` syntax from §7.3.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalingListData {
    /// Matrix entries in syntax-table order.
    pub matrices: Vec<ScalingListMatrix>,
}

impl ScalingListData {
    /// Parses all 20 scaling-list matrix entries.
    pub fn parse(reader: &mut BitReader<'_>) -> Result<Self, SyntaxError> {
        let mut matrices = Vec::with_capacity(20);
        for size_id in 0..4u8 {
            let matrix_step = if size_id == 3 { 3 } else { 1 };
            for matrix_id in (0..6u8).step_by(matrix_step) {
                let pred_mode_flag = reader.read_u(1)? != 0;
                if !pred_mode_flag {
                    matrices.push(ScalingListMatrix {
                        size_id,
                        matrix_id,
                        pred_mode_flag,
                        pred_matrix_id_delta: Some(reader.read_ue()?),
                        dc_coef_minus8: None,
                        delta_coefficients: Vec::new(),
                        coefficients: Vec::new(),
                    });
                    continue;
                }

                let coefficient_count = usize::min(64, 1usize << (4 + (usize::from(size_id) << 1)));
                let dc_coef_minus8 = if size_id > 1 {
                    Some(reader.read_se()?)
                } else {
                    None
                };
                let mut next_coefficient = dc_coef_minus8.unwrap_or(0) + 8;
                let mut delta_coefficients = Vec::with_capacity(coefficient_count);
                let mut coefficients = Vec::with_capacity(coefficient_count);
                if size_id <= 1 {
                    next_coefficient = 8;
                }
                for _ in 0..coefficient_count {
                    let delta = reader.read_se()?;
                    next_coefficient = (next_coefficient + delta + 256).rem_euclid(256);
                    delta_coefficients.push(delta);
                    coefficients.push(next_coefficient as u8);
                }
                matrices.push(ScalingListMatrix {
                    size_id,
                    matrix_id,
                    pred_mode_flag,
                    pred_matrix_id_delta: None,
                    dc_coef_minus8,
                    delta_coefficients,
                    coefficients,
                });
            }
        }
        Ok(Self { matrices })
    }
}
