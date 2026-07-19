//! Decode HEVC samples from an MP4 with `h265rs` and write selected frames as JPEG.
//!
//! The MP4 code below is only a small ISO BMFF sample-table reader. HEVC NAL
//! parsing, CABAC, slice parsing, reconstruction, POC, and DPB handling all
//! go through `h265rs`; `jpeg-encoder` is used only for the final JPEG.
//!
//! Usage:
//!
//! ```text
//! cargo run --release --example every_fifth_second_jpg -- \
//!     output.mp4 every_fifth_second_frames
//! ```

use std::error::Error;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use h265rs::{CompleteHevcDecoder, CompleteHevcFrame};
use jpeg_encoder::{ColorType, Encoder};

const FRAME_RATE: u64 = 24;
const INTERVAL_SECONDS: u64 = 5;
const JPEG_QUALITY: u8 = 90;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).map_or("output.mp4", String::as_str);
    let output_dir = args
        .get(2)
        .map_or("every_fifth_second_frames", String::as_str);
    let frame_rate = args
        .get(3)
        .map_or(Ok(FRAME_RATE), |value| value.parse::<u64>())?;
    if frame_rate == 0 {
        return Err("frame rate must be greater than zero".into());
    }

    decode_video(input, output_dir, frame_rate)
}

fn decode_video(input: &str, output_dir: &str, frame_rate: u64) -> Result<(), Box<dyn Error>> {
    let mut reader = Mp4HevcReader::open(Path::new(input))?;
    let mut decoder = CompleteHevcDecoder::new();
    for (index, nal) in reader.parameter_nals.iter().enumerate() {
        decode_raw_nal(&mut decoder, nal).map_err(|error| {
            format!(
                "hvcC parameter NAL {index} (type {}) failed: {error}",
                nal_type(nal)
            )
        })?;
    }
    fs::create_dir_all(output_dir)?;

    let frames_per_output = frame_rate
        .checked_mul(INTERVAL_SECONDS)
        .ok_or("frame rate is too large")?;
    let mut decoded_frames = 0u64;
    let mut output_index = 0u64;
    let mut skipped_nals = 0u64;
    let mut first_skipped_sample = None;

    println!("decoding {input} -> {output_dir} with h265rs at {frame_rate} fps");
    while let Some(sample) = reader.next_sample()? {
        let sample_number = reader.sample_index;
        for nal in split_length_prefixed_nals(&sample, reader.nal_length_size)? {
            let frame = match decode_raw_nal(&mut decoder, nal) {
                Ok(frame) => frame,
                Err(_error) => {
                    skipped_nals += 1;
                    first_skipped_sample.get_or_insert(sample_number);
                    continue;
                }
            };
            let Some(frame) = frame else {
                continue;
            };
            if decoded_frames.is_multiple_of(frames_per_output) {
                let (rgb, width, height) = frame_to_rgb8(&frame)?;
                let path = frame_path(output_dir, output_index);
                write_jpeg(&rgb, width, height, &path)?;
                println!(
                    "wrote {} at {:.3}s{}",
                    path.display(),
                    decoded_frames as f64 / frame_rate as f64,
                    if nal_type(nal) >= 16 && nal_type(nal) <= 23 {
                        " (IRAP)"
                    } else {
                        ""
                    }
                );
                output_index += 1;
            }
            decoded_frames += 1;
        }
    }

    println!("decoded {decoded_frames} frame(s), wrote {output_index} JPEG frame(s)");
    if skipped_nals != 0 {
        println!(
            "skipped {skipped_nals} undecodable NAL(s) while recovering; first at MP4 sample {}",
            first_skipped_sample.expect("skipped_nals is non-zero")
        );
    }
    Ok(())
}

fn decode_raw_nal(
    decoder: &mut CompleteHevcDecoder,
    nal: &[u8],
) -> Result<Option<CompleteHevcFrame>, Box<dyn Error>> {
    let mut annex_b = Vec::with_capacity(nal.len() + 4);
    annex_b.extend_from_slice(&[0, 0, 0, 1]);
    annex_b.extend_from_slice(nal);
    let parsed = h265rs::rust_h265::parse_annex_b(&annex_b);
    let parsed = parsed.first().ok_or("h265rs could not parse MP4 NAL")?;
    Ok(decoder.decode_nal(parsed)?)
}

fn split_length_prefixed_nals(
    sample: &[u8],
    nal_length_size: usize,
) -> Result<Vec<&[u8]>, Box<dyn Error>> {
    if !(1..=4).contains(&nal_length_size) {
        return Err("NAL length size must be 1..=4".into());
    }
    let mut nals = Vec::new();
    let mut cursor = 0usize;
    while cursor < sample.len() {
        let length_end = cursor
            .checked_add(nal_length_size)
            .ok_or("NAL length offset overflows")?;
        if length_end > sample.len() {
            return Err("truncated length-prefixed NAL size".into());
        }
        let length = sample[cursor..length_end]
            .iter()
            .fold(0usize, |value, &byte| (value << 8) | usize::from(byte));
        cursor = length_end;
        if length == 0 {
            return Err("length-prefixed sample contains an empty NAL".into());
        }
        let end = cursor
            .checked_add(length)
            .ok_or("NAL payload offset overflows")?;
        if end > sample.len() {
            return Err("truncated length-prefixed NAL payload".into());
        }
        nals.push(&sample[cursor..end]);
        cursor = end;
    }
    Ok(nals)
}

fn frame_path(output_dir: &str, index: u64) -> PathBuf {
    Path::new(output_dir).join(format!("frame_{index:06}.jpg"))
}

fn nal_type(nal: &[u8]) -> u8 {
    nal.first().map_or(0, |byte| (byte >> 1) & 0x3f)
}

fn write_jpeg(rgb: &[u8], width: usize, height: usize, path: &Path) -> Result<(), Box<dyn Error>> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or("frame dimensions overflow")?;
    if rgb.len() != expected_len {
        return Err(format!(
            "decoder returned {} RGB bytes for a {width}x{height} frame",
            rgb.len()
        )
        .into());
    }

    let output = File::create(path)?;
    let mut writer = BufWriter::new(output);
    Encoder::new(&mut writer, JPEG_QUALITY).encode(
        rgb,
        u16::try_from(width)?,
        u16::try_from(height)?,
        ColorType::Rgb,
    )?;
    Ok(())
}

fn frame_to_rgb8(frame: &CompleteHevcFrame) -> Result<(Vec<u8>, usize, usize), Box<dyn Error>> {
    let y_plane = frame.y.as_u8().ok_or("h265rs returned a non-8-bit frame")?;
    let cb_plane = frame.u.as_u8().ok_or("h265rs returned non-8-bit chroma")?;
    let cr_plane = frame.v.as_u8().ok_or("h265rs returned non-8-bit chroma")?;
    let width = frame.width as usize;
    let height = frame.height as usize;
    let chroma_width = width.div_ceil(2);
    let mut rgb = Vec::with_capacity(width * height * 3);

    for y in 0..height {
        for x in 0..width {
            let y_value = y_plane[y * width + x] as f32;
            let chroma_index = (y / 2) * chroma_width + x / 2;
            let cb_value = cb_plane[chroma_index] as f32 - 128.0;
            let cr_value = cr_plane[chroma_index] as f32 - 128.0;
            rgb.push((y_value + 1.402 * cr_value).round().clamp(0.0, 255.0) as u8);
            rgb.push(
                (y_value - 0.344_136 * cb_value - 0.714_136 * cr_value)
                    .round()
                    .clamp(0.0, 255.0) as u8,
            );
            rgb.push((y_value + 1.772 * cb_value).round().clamp(0.0, 255.0) as u8);
        }
    }
    Ok((rgb, width, height))
}

struct Mp4HevcReader {
    file: File,
    samples: Vec<(u64, u32)>,
    parameter_nals: Vec<Vec<u8>>,
    nal_length_size: usize,
    sample_index: usize,
}

impl Mp4HevcReader {
    fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let moov = read_moov(&mut file, file_size)?;
        if !moov.windows(4).any(|tag| tag == b"hvc1" || tag == b"hev1") {
            return Err("MP4 does not contain an HEVC video track".into());
        }
        let video_trak = find_video_trak(&moov).ok_or("MP4 has no video track")?;
        let chunk_offsets = find_box_data(video_trak, b"co64")
            .map(parse_co64)
            .or_else(|| find_box_data(video_trak, b"stco").map(parse_stco))
            .ok_or("MP4 has no stco/co64 box")?;
        let sample_sizes = find_box_data(video_trak, b"stsz")
            .map(parse_stsz)
            .ok_or("MP4 has no stsz box")?;
        let stsc = find_box_data(video_trak, b"stsc")
            .map(parse_stsc)
            .unwrap_or_else(|| vec![(1, 1)]);
        let samples = build_sample_table(&chunk_offsets, &sample_sizes, &stsc)?;
        let hvcc = find_box_data(&moov, b"hvcC").ok_or("MP4 has no hvcC box")?;
        if hvcc.len() < 23 {
            return Err("MP4 hvcC box is truncated".into());
        }
        let nal_length_size = usize::from(hvcc[21] & 3) + 1;
        let parameter_nals = parse_hvcc_nals(hvcc)?;
        Ok(Self {
            file,
            samples,
            parameter_nals,
            nal_length_size,
            sample_index: 0,
        })
    }

    fn next_sample(&mut self) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        let Some(&(offset, size)) = self.samples.get(self.sample_index) else {
            return Ok(None);
        };
        self.sample_index += 1;
        self.file.seek(SeekFrom::Start(offset))?;
        let mut sample = vec![0u8; size as usize];
        self.file.read_exact(&mut sample)?;
        Ok(Some(sample))
    }
}

fn read_moov(file: &mut File, file_size: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut position = 0u64;
    while position + 8 <= file_size {
        file.seek(SeekFrom::Start(position))?;
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        let size32 = u32::from_be_bytes(header[..4].try_into()?);
        let header_size = if size32 == 1 { 16 } else { 8 };
        let size = if size32 == 1 {
            let mut extended = [0u8; 8];
            file.read_exact(&mut extended)?;
            u64::from_be_bytes(extended)
        } else if size32 == 0 {
            file_size - position
        } else {
            u64::from(size32)
        };
        if size < header_size || position + size > file_size {
            return Err("invalid MP4 box size".into());
        }
        if &header[4..8] == b"moov" {
            let content_size = usize::try_from(size - header_size)?;
            let mut content = vec![0u8; content_size];
            file.seek(SeekFrom::Start(position + header_size))?;
            file.read_exact(&mut content)?;
            return Ok(content);
        }
        position += size;
    }
    Err("MP4 has no moov box".into())
}

fn find_box_data<'a>(data: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    for (tag_position, window) in data.windows(4).enumerate() {
        if window != tag || tag_position < 4 {
            continue;
        }
        let position = tag_position - 4;
        let size32 = u32::from_be_bytes(data[position..tag_position].try_into().ok()?) as usize;
        let header_size = if size32 == 1 { 16 } else { 8 };
        if position + header_size > data.len() {
            continue;
        }
        let size = if size32 == 1 {
            usize::try_from(u64::from_be_bytes(
                data[position + 8..position + 16].try_into().ok()?,
            ))
            .ok()?
        } else if size32 == 0 {
            data.len() - position
        } else {
            size32
        };
        if size >= header_size && position + size <= data.len() {
            return Some(&data[position + header_size..position + size]);
        }
    }
    None
}

fn find_video_trak(moov: &[u8]) -> Option<&[u8]> {
    let mut position = 0usize;
    while position + 8 <= moov.len() {
        let size = u32::from_be_bytes(moov[position..position + 4].try_into().ok()?) as usize;
        if size < 8 || position + size > moov.len() {
            return None;
        }
        if &moov[position + 4..position + 8] == b"trak" {
            let trak = &moov[position + 8..position + size];
            if find_box_data(trak, b"vmhd").is_some() {
                return Some(trak);
            }
        }
        position += size;
    }
    None
}

fn parse_stco(data: &[u8]) -> Vec<u64> {
    if data.len() < 8 {
        return Vec::new();
    }
    let count = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
    (0..count)
        .filter_map(|index| {
            let start = 8 + index * 4;
            data.get(start..start + 4)
                .map(|bytes| u64::from(u32::from_be_bytes(bytes.try_into().unwrap())))
        })
        .collect()
}

fn parse_co64(data: &[u8]) -> Vec<u64> {
    if data.len() < 8 {
        return Vec::new();
    }
    let count = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
    (0..count)
        .filter_map(|index| {
            let start = 8 + index * 8;
            data.get(start..start + 8)
                .map(|bytes| u64::from_be_bytes(bytes.try_into().unwrap()))
        })
        .collect()
}

fn parse_stsz(data: &[u8]) -> Vec<u32> {
    if data.len() < 12 {
        return Vec::new();
    }
    let default_size = u32::from_be_bytes(data[4..8].try_into().unwrap());
    let count = u32::from_be_bytes(data[8..12].try_into().unwrap()) as usize;
    if default_size != 0 {
        return vec![default_size; count];
    }
    (0..count)
        .filter_map(|index| {
            let start = 12 + index * 4;
            data.get(start..start + 4)
                .map(|bytes| u32::from_be_bytes(bytes.try_into().unwrap()))
        })
        .collect()
}

fn parse_stsc(data: &[u8]) -> Vec<(u32, u32)> {
    if data.len() < 8 {
        return Vec::new();
    }
    let count = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
    (0..count)
        .filter_map(|index| {
            let start = 8 + index * 12;
            let bytes = data.get(start..start + 12)?;
            Some((
                u32::from_be_bytes(bytes[0..4].try_into().unwrap()),
                u32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            ))
        })
        .collect()
}

fn build_sample_table(
    chunk_offsets: &[u64],
    sample_sizes: &[u32],
    stsc: &[(u32, u32)],
) -> Result<Vec<(u64, u32)>, Box<dyn Error>> {
    let mut table = Vec::with_capacity(sample_sizes.len());
    let mut sample_index = 0usize;
    for (chunk_index, &chunk_offset) in chunk_offsets.iter().enumerate() {
        let chunk_number = u32::try_from(chunk_index + 1)?;
        let samples_per_chunk = stsc
            .iter()
            .rev()
            .find(|&&(first_chunk, _)| chunk_number >= first_chunk)
            .map_or(1, |&(_, samples)| samples);
        let mut offset = chunk_offset;
        for _ in 0..samples_per_chunk {
            let Some(&size) = sample_sizes.get(sample_index) else {
                break;
            };
            table.push((offset, size));
            offset = offset
                .checked_add(u64::from(size))
                .ok_or("sample offset overflow")?;
            sample_index += 1;
        }
    }
    Ok(table)
}

fn parse_hvcc_nals(hvcc: &[u8]) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    if hvcc.len() < 23 {
        return Err("hvcC box is truncated".into());
    }
    let array_count = hvcc[22] as usize;
    let mut position = 23usize;
    let mut nals = Vec::new();
    for _ in 0..array_count {
        if position + 3 > hvcc.len() {
            return Err("hvcC array header is truncated".into());
        }
        let count = u16::from_be_bytes(hvcc[position + 1..position + 3].try_into()?) as usize;
        position += 3;
        for _ in 0..count {
            if position + 2 > hvcc.len() {
                return Err("hvcC NAL length is truncated".into());
            }
            let length = u16::from_be_bytes(hvcc[position..position + 2].try_into()?) as usize;
            position += 2;
            let end = position
                .checked_add(length)
                .ok_or("hvcC NAL length overflow")?;
            if end > hvcc.len() {
                return Err("hvcC NAL payload is truncated".into());
            }
            nals.push(hvcc[position..end].to_vec());
            position = end;
        }
    }
    Ok(nals)
}
