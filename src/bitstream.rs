/// Prefixes each NAL unit with the three-byte Annex B start-code prefix.
///
/// The returned bytes are a byte-stream framing of the supplied logical NAL
/// unit stream. Emulation-prevention bytes are intentionally outside this
/// helper; they belong to RBSP-to-NAL conversion.
pub fn nal_units_to_byte_stream(nal_units: &[&[u8]]) -> Vec<u8> {
    let payload_size = nal_units.iter().map(|unit| unit.len() + 3).sum();
    let mut stream = Vec::with_capacity(payload_size);
    for unit in nal_units {
        stream.extend_from_slice(&[0, 0, 1]);
        stream.extend_from_slice(unit);
    }
    stream
}

/// Extracts NAL-unit payloads from a byte stream framed with Annex B start
/// codes. Invalid or empty units are skipped.
pub fn nal_units_from_byte_stream(stream: &[u8]) -> Vec<&[u8]> {
    let mut units = Vec::new();
    let mut cursor = 0;
    while let Some((prefix_start, payload_start)) = find_start_code(stream, cursor) {
        let next = find_start_code(stream, payload_start);
        let mut end = next.map_or(stream.len(), |(start, _)| start);
        while end > payload_start && stream[end - 1] == 0 {
            end -= 1;
        }
        if payload_start < end {
            units.push(&stream[payload_start..end]);
        }
        cursor = next.map_or(stream.len(), |(start, _)| start);
        if cursor <= prefix_start {
            break;
        }
    }
    units
}

fn find_start_code(stream: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut index = from;
    while index + 2 < stream.len() {
        if stream[index] == 0 && stream[index + 1] == 0 {
            if stream[index + 2] == 1 {
                return Some((index, index + 3));
            }
            if index + 3 < stream.len() && stream[index + 2] == 0 && stream[index + 3] == 1 {
                return Some((index, index + 4));
            }
        }
        index += 1;
    }
    None
}
