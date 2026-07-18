use super::{BitReader, SyntaxError};

/// One short-term reference picture set from §7.3.7.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortTermReferencePictureSet {
    /// RPS index in the containing syntax structure.
    pub index: usize,
    /// `inter_ref_pic_set_prediction_flag`.
    pub inter_ref_pic_set_prediction_flag: bool,
    /// Referenced RPS index for inter-predicted sets.
    pub reference_rps_idx: Option<usize>,
    /// `delta_idx_minus1`, present for a slice-created RPS.
    pub delta_idx_minus1: Option<u64>,
    /// `delta_rps_sign`.
    pub delta_rps_sign: Option<bool>,
    /// `abs_delta_rps_minus1`.
    pub abs_delta_rps_minus1: Option<u64>,
    /// `used_by_curr_pic_flag` values for inter-predicted sets.
    pub used_by_curr_pic_flag: Vec<bool>,
    /// `use_delta_flag` values for inter-predicted sets.
    pub use_delta_flag: Vec<bool>,
    /// `num_negative_pics` for directly coded sets.
    pub num_negative_pics: Option<u64>,
    /// `num_positive_pics` for directly coded sets.
    pub num_positive_pics: Option<u64>,
    /// `delta_poc_s0_minus1` values.
    pub delta_poc_s0_minus1: Vec<u64>,
    /// `used_by_curr_pic_s0_flag` values.
    pub used_by_curr_pic_s0_flag: Vec<bool>,
    /// `delta_poc_s1_minus1` values.
    pub delta_poc_s1_minus1: Vec<u64>,
    /// `used_by_curr_pic_s1_flag` values.
    pub used_by_curr_pic_s1_flag: Vec<bool>,
    /// Derived `NumDeltaPocs` for this set.
    pub num_delta_pocs: usize,
}

/// Parses one `st_ref_pic_set(stRpsIdx)` syntax structure.
pub fn parse_short_term_reference_picture_set(
    reader: &mut BitReader<'_>,
    index: usize,
    previous_sets: &[ShortTermReferencePictureSet],
    num_short_term_ref_pic_sets: usize,
) -> Result<ShortTermReferencePictureSet, SyntaxError> {
    let inter_ref_pic_set_prediction_flag = if index != 0 {
        reader.read_u(1)? != 0
    } else {
        false
    };
    if inter_ref_pic_set_prediction_flag {
        let delta_idx_minus1 = if index == num_short_term_ref_pic_sets {
            Some(reader.read_ue()?)
        } else {
            None
        };
        let delta_idx = usize::try_from(delta_idx_minus1.unwrap_or(0))
            .map_err(|_| SyntaxError::InvalidSyntaxValue("delta_idx_minus1 is too large"))?;
        let reference_rps_idx = index
            .checked_sub(
                delta_idx
                    .checked_add(1)
                    .ok_or(SyntaxError::InvalidSyntaxValue(
                        "delta_idx_minus1 is too large",
                    ))?,
            )
            .ok_or(SyntaxError::InvalidSyntaxValue(
                "referenced short-term RPS index is out of range",
            ))?;
        let reference =
            previous_sets
                .get(reference_rps_idx)
                .ok_or(SyntaxError::InvalidSyntaxValue(
                    "referenced short-term RPS has not been parsed",
                ))?;
        let delta_rps_sign = reader.read_u(1)? != 0;
        let abs_delta_rps_minus1 = reader.read_ue()?;
        let mut used_by_curr_pic_flag = Vec::with_capacity(reference.num_delta_pocs + 1);
        let mut use_delta_flag = Vec::with_capacity(reference.num_delta_pocs + 1);
        for _ in 0..=reference.num_delta_pocs {
            let used = reader.read_u(1)? != 0;
            let use_delta = if used { false } else { reader.read_u(1)? != 0 };
            used_by_curr_pic_flag.push(used);
            use_delta_flag.push(use_delta);
        }
        let num_delta_pocs = used_by_curr_pic_flag
            .iter()
            .zip(&use_delta_flag)
            .filter(|(used, use_delta)| **used || **use_delta)
            .count();
        Ok(ShortTermReferencePictureSet {
            index,
            inter_ref_pic_set_prediction_flag,
            reference_rps_idx: Some(reference_rps_idx),
            delta_idx_minus1,
            delta_rps_sign: Some(delta_rps_sign),
            abs_delta_rps_minus1: Some(abs_delta_rps_minus1),
            used_by_curr_pic_flag,
            use_delta_flag,
            num_negative_pics: None,
            num_positive_pics: None,
            delta_poc_s0_minus1: Vec::new(),
            used_by_curr_pic_s0_flag: Vec::new(),
            delta_poc_s1_minus1: Vec::new(),
            used_by_curr_pic_s1_flag: Vec::new(),
            num_delta_pocs,
        })
    } else {
        let num_negative_pics = reader.read_ue()?;
        let num_positive_pics = reader.read_ue()?;
        let num_delta_pocs = num_negative_pics
            .checked_add(num_positive_pics)
            .ok_or(SyntaxError::InvalidSyntaxValue("too many delta POCs"))?;
        let mut delta_poc_s0_minus1 = Vec::with_capacity(num_negative_pics as usize);
        let mut used_by_curr_pic_s0_flag = Vec::with_capacity(num_negative_pics as usize);
        for _ in 0..num_negative_pics {
            delta_poc_s0_minus1.push(reader.read_ue()?);
            used_by_curr_pic_s0_flag.push(reader.read_u(1)? != 0);
        }
        let mut delta_poc_s1_minus1 = Vec::with_capacity(num_positive_pics as usize);
        let mut used_by_curr_pic_s1_flag = Vec::with_capacity(num_positive_pics as usize);
        for _ in 0..num_positive_pics {
            delta_poc_s1_minus1.push(reader.read_ue()?);
            used_by_curr_pic_s1_flag.push(reader.read_u(1)? != 0);
        }
        Ok(ShortTermReferencePictureSet {
            index,
            inter_ref_pic_set_prediction_flag,
            reference_rps_idx: None,
            delta_idx_minus1: None,
            delta_rps_sign: None,
            abs_delta_rps_minus1: None,
            used_by_curr_pic_flag: Vec::new(),
            use_delta_flag: Vec::new(),
            num_negative_pics: Some(num_negative_pics),
            num_positive_pics: Some(num_positive_pics),
            delta_poc_s0_minus1,
            used_by_curr_pic_s0_flag,
            delta_poc_s1_minus1,
            used_by_curr_pic_s1_flag,
            num_delta_pocs: usize::try_from(num_delta_pocs)
                .map_err(|_| SyntaxError::InvalidSyntaxValue("too many delta POCs"))?,
        })
    }
}
