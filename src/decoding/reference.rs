use super::{DecodedPicture, SamplePlane};
use crate::syntax::{ShortTermReferencePictureSet, SliceReferencePictureSet};

/// Derives `PicOrderCntMsb` and `PicOrderCntVal` from §8.3.1.
pub fn derive_picture_order_count(
    slice_pic_order_cnt_lsb: u64,
    prev_pic_order_cnt_lsb: u64,
    prev_pic_order_cnt_msb: i64,
    max_pic_order_cnt_lsb: u64,
    no_rasl_output: bool,
) -> i64 {
    if no_rasl_output {
        return 0;
    }
    let half = max_pic_order_cnt_lsb / 2;
    let msb = if slice_pic_order_cnt_lsb < prev_pic_order_cnt_lsb
        && prev_pic_order_cnt_lsb - slice_pic_order_cnt_lsb >= half
    {
        prev_pic_order_cnt_msb + max_pic_order_cnt_lsb as i64
    } else if slice_pic_order_cnt_lsb > prev_pic_order_cnt_lsb
        && slice_pic_order_cnt_lsb - prev_pic_order_cnt_lsb > half
    {
        prev_pic_order_cnt_msb - max_pic_order_cnt_lsb as i64
    } else {
        prev_pic_order_cnt_msb
    };
    msb + slice_pic_order_cnt_lsb as i64
}

/// Returns `PicOrderCnt(picA) - PicOrderCnt(picB)`.
pub const fn diff_picture_order_count(pic_a: i64, pic_b: i64) -> i64 {
    pic_a - pic_b
}

/// DPB marking state from §8.3.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PictureMarking {
    /// Not used for reference.
    Unused,
    /// Used for short-term reference.
    ShortTerm,
    /// Used for long-term reference.
    LongTerm,
}

/// A decoded picture stored in the decoded-picture buffer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceSet {
    /// Picture sample data.
    pub picture: DecodedPicture,
    /// Full picture order count, when known.
    pub picture_order_count: i64,
    /// Layer identifier.
    pub layer_id: u8,
    /// Reference marking.
    pub marking: PictureMarking,
    /// Whether the picture is output eligible.
    pub output_needed: bool,
}

/// A bounded decoded-picture buffer with the marking transitions of §8.3.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedPictureBuffer {
    capacity: usize,
    pictures: Vec<ReferenceSet>,
}

impl DecodedPictureBuffer {
    /// Creates an empty DPB.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            pictures: Vec::new(),
        }
    }
    /// Maximum number of stored pictures.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }
    /// Current entries.
    pub fn pictures(&self) -> &[ReferenceSet] {
        &self.pictures
    }
    /// Inserts a decoded picture, evicting the oldest unused/output picture if required.
    pub fn insert(&mut self, picture: ReferenceSet) {
        if self.capacity == 0 {
            return;
        }
        if self.pictures.len() >= self.capacity {
            if let Some(index) = self
                .pictures
                .iter()
                .position(|item| item.marking == PictureMarking::Unused && !item.output_needed)
            {
                self.pictures.remove(index);
            } else if !self.pictures.is_empty() {
                self.pictures.remove(0);
            }
        }
        self.pictures.push(picture);
    }
    /// Marks all pictures in a layer as unused, as required at an IRAP boundary.
    pub fn mark_layer_unused(&mut self, layer_id: u8) {
        for item in &mut self.pictures {
            if item.layer_id == layer_id {
                item.marking = PictureMarking::Unused;
            }
        }
    }
    /// Applies a compact RPS marking operation for the current layer.
    pub fn apply_reference_set(&mut self, current_poc: i64, layer_id: u8, rps: &ReferenceSet) {
        for item in &mut self.pictures {
            if item.layer_id != layer_id {
                continue;
            }
            item.marking = if item.picture_order_count == rps.picture_order_count {
                PictureMarking::ShortTerm
            } else if item.picture_order_count < current_poc {
                PictureMarking::Unused
            } else {
                item.marking
            };
        }
    }
    /// Marks the absolute short-/long-term POC sets signalled for a picture.
    pub fn mark_reference_pocs(
        &mut self,
        current_poc: i64,
        layer_id: u8,
        short_term: &[i64],
        long_term: &[i64],
    ) {
        for item in &mut self.pictures {
            if item.layer_id != layer_id || item.picture_order_count == current_poc {
                continue;
            }
            item.marking = if long_term.contains(&item.picture_order_count) {
                PictureMarking::LongTerm
            } else if short_term.contains(&item.picture_order_count) {
                PictureMarking::ShortTerm
            } else {
                PictureMarking::Unused
            };
        }
    }
    /// Returns references with a matching marking and layer, sorted by POC.
    pub fn references(&self, layer_id: u8, marking: PictureMarking) -> Vec<&ReferenceSet> {
        let mut result: Vec<_> = self
            .pictures
            .iter()
            .filter(|item| item.layer_id == layer_id && item.marking == marking)
            .collect();
        result.sort_by_key(|item| item.picture_order_count);
        result
    }
}

/// The two reference lists constructed by §8.3.4.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferencePictureLists {
    /// List 0 POC values in reference-index order.
    pub list0: Vec<i64>,
    /// List 1 POC values in reference-index order.
    pub list1: Vec<i64>,
}

/// Builds reference lists from the standard's negative/positive short-term
/// POC lists and long-term entries.  The returned values are POC values; a
/// caller maps them to DPB entries, preserving the signalled list order.
pub fn build_reference_picture_lists(
    current_poc: i64,
    negative: &[i64],
    positive: &[i64],
    long_term: &[i64],
    list0_length: usize,
    list1_length: usize,
) -> ReferencePictureLists {
    let mut list0: Vec<i64> = negative
        .iter()
        .chain(positive)
        .chain(long_term)
        .copied()
        .collect();
    let mut list1: Vec<i64> = positive
        .iter()
        .chain(negative)
        .chain(long_term)
        .copied()
        .collect();
    let _ = current_poc;
    list0.truncate(list0_length);
    list1.truncate(list1_length);
    ReferencePictureLists { list0, list1 }
}

/// Derives the collocated picture and `NoBackwardPredFlag` (§8.3.5).
pub fn derive_collocated_picture_and_no_backward_prediction_flag(
    current_poc: i64,
    is_b_slice: bool,
    collocated_from_l0: bool,
    collocated_ref_idx: usize,
    list0: &[i64],
    list1: &[i64],
) -> Option<(i64, bool)> {
    let collocated_list = if collocated_from_l0 { list0 } else { list1 };
    let collocated = *collocated_list.get(collocated_ref_idx)?;
    let no_backward = is_b_slice && list1.iter().all(|&poc| poc <= current_poc);
    Some((collocated, no_backward))
}

/// Creates an unavailable reference picture by copying the nearest available
/// sample value, as specified by §8.3.3.2.
pub fn generate_unavailable_picture(format: crate::PictureFormat, value: i32) -> DecodedPicture {
    let mut picture = DecodedPicture::new(format);
    for component in 0..format.component_count() {
        if let Some(dimension) = format.component_dimension(component) {
            picture.set_plane(
                component,
                SamplePlane::new(dimension.width, dimension.height, value),
            );
        }
    }
    picture
}

/// Derives the five POC lists from a directly coded short-term RPS.
pub fn derive_reference_set(
    current_poc: i64,
    rps: &ShortTermReferencePictureSet,
) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
    let mut negative = Vec::new();
    let mut positive = Vec::new();
    let mut following = Vec::new();
    if let (Some(negative_count), Some(positive_count)) =
        (rps.num_negative_pics, rps.num_positive_pics)
    {
        let mut delta = 0_i64;
        for index in 0..negative_count as usize {
            delta -= rps.delta_poc_s0_minus1[index] as i64 + 1;
            if rps.used_by_curr_pic_s0_flag[index] {
                negative.push(current_poc + delta);
            } else {
                following.push(current_poc + delta);
            }
        }
        delta = 0;
        for index in 0..positive_count as usize {
            delta += rps.delta_poc_s1_minus1[index] as i64 + 1;
            if rps.used_by_curr_pic_s1_flag[index] {
                positive.push(current_poc + delta);
            } else {
                following.push(current_poc + delta);
            }
        }
    }
    (negative, positive, following)
}

/// Derives the short-term current/following POC lists for an SPS RPS or an
/// inter-predicted RPS (§8.3.2).  The result preserves the negative-before and
/// positive-after ordering required by reference-list construction.
pub fn derive_reference_set_from_sets(
    current_poc: i64,
    sets: &[ShortTermReferencePictureSet],
    index: usize,
) -> Option<(Vec<i64>, Vec<i64>, Vec<i64>)> {
    let rps = sets.get(index)?;
    if rps.num_negative_pics.is_some() {
        return Some(derive_reference_set(current_poc, rps));
    }
    let reference_index = rps.reference_rps_idx?;
    let reference = sets.get(reference_index)?;
    let (reference_negative, reference_positive, reference_following) =
        if reference.num_negative_pics.is_some() {
            derive_reference_set(current_poc, reference)
        } else {
            derive_reference_set_from_sets(current_poc, sets, reference_index)?
        };
    let mut deltas: Vec<i64> = reference_negative
        .iter()
        .chain(&reference_positive)
        .chain(&reference_following)
        .map(|poc| *poc - current_poc)
        .collect();
    let delta_rps = if rps.delta_rps_sign.unwrap_or(false) {
        -1
    } else {
        1
    } * (rps.abs_delta_rps_minus1.unwrap_or(0) as i64 + 1);
    deltas.push(delta_rps);
    let mut negative = Vec::new();
    let mut positive = Vec::new();
    let mut following = Vec::new();
    for (index, delta) in deltas.into_iter().enumerate() {
        let used = rps
            .used_by_curr_pic_flag
            .get(index)
            .copied()
            .unwrap_or(false);
        let use_delta = rps.use_delta_flag.get(index).copied().unwrap_or(false);
        if !used && !use_delta {
            continue;
        }
        let poc = current_poc + delta;
        if delta < 0 {
            if used {
                negative.push(poc);
            } else {
                following.push(poc);
            }
        } else if delta > 0 {
            if used {
                positive.push(poc);
            } else {
                following.push(poc);
            }
        } else if used {
            // A zero delta is not a conforming reference picture, but keeping
            // it in the following list makes this helper total for best-effort
            // decoding and lets the caller perform conformance reporting.
            following.push(poc);
        }
    }
    Some((negative, positive, following))
}

/// Extracts a slice-created short-term RPS from the slice syntax.
pub fn slice_short_term_reference_set(
    slice: &SliceReferencePictureSet,
) -> Option<&ShortTermReferencePictureSet> {
    slice.short_term_ref_pic_set.as_ref()
}
