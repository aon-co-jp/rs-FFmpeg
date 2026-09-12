//! 最小構成のWAV(PCM)リーダー/ライター。
//!
//! FFmpegへの敬意を込めたRust版(`rs-ffmpeg`)の最初の一歩として、
//! 非圧縮PCM WAVの読み込み・書き出し・簡易リサンプル/ビット深度変換/
//! チャンネル数変換のみを実装する。MP3/AAC/FLAC等の圧縮コーデックや
//! 動画は現時点では対象外(README/CLAUDE.mdに明記)。

use std::fs;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WavInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub num_samples_per_channel: u32,
}

impl WavInfo {
    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.num_samples_per_channel as f64 / self.sample_rate as f64
    }
}

pub struct WavData {
    pub info: WavInfo,
    /// チャンネルインターリーブされた正規化サンプル(-1.0..=1.0)。
    pub samples: Vec<f32>,
}

pub fn probe(path: &Path) -> io::Result<WavInfo> {
    Ok(read(path)?.info)
}

pub fn read(path: &Path) -> io::Result<WavData> {
    let bytes = fs::read(path)?;
    parse(&bytes)
}

fn parse(bytes: &[u8]) -> io::Result<WavData> {
    let err = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_string());

    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(err("WAVファイルとして認識できません(RIFF/WAVEヘッダが無い) / not a recognizable WAV file"));
    }

    let mut pos = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (format_tag, channels, sample_rate, bits_per_sample)
    let mut data: Option<&[u8]> = None;

    while pos + 8 <= bytes.len() {
        let chunk_id = &bytes[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + chunk_size).min(bytes.len());
        let body = &bytes[body_start..body_end];

        if chunk_id == b"fmt " {
            if body.len() < 16 {
                return Err(err("fmtチャンクが不正です / malformed fmt chunk"));
            }
            let format_tag = u16::from_le_bytes(body[0..2].try_into().unwrap());
            let channels = u16::from_le_bytes(body[2..4].try_into().unwrap());
            let sample_rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
            let bits_per_sample = u16::from_le_bytes(body[14..16].try_into().unwrap());
            fmt = Some((format_tag, channels, sample_rate, bits_per_sample));
        } else if chunk_id == b"data" {
            data = Some(body);
        }

        pos = body_start + chunk_size + (chunk_size % 2); // チャンクは偶数境界
    }

    let (format_tag, channels, sample_rate, bits_per_sample) =
        fmt.ok_or_else(|| err("fmtチャンクが見つかりません / fmt chunk not found"))?;
    let data = data.ok_or_else(|| err("dataチャンクが見つかりません / data chunk not found"))?;

    if format_tag != 1 && format_tag != 0xFFFE {
        return Err(err("PCM以外のWAV(圧縮形式)はv0.1では未対応です / non-PCM WAV is not yet supported"));
    }
    if bits_per_sample != 8 && bits_per_sample != 16 {
        return Err(err("8bit/16bit以外のビット深度はv0.1では未対応です / only 8-bit and 16-bit PCM are supported"));
    }

    let bytes_per_sample = (bits_per_sample / 8) as usize;
    let frame_size = bytes_per_sample * channels as usize;
    let num_frames = if frame_size == 0 { 0 } else { data.len() / frame_size };

    let mut samples = Vec::with_capacity(num_frames * channels as usize);
    for frame in data.chunks_exact(frame_size.max(1)) {
        for ch in frame.chunks_exact(bytes_per_sample) {
            let normalized = match bits_per_sample {
                8 => (ch[0] as f32 - 128.0) / 128.0,
                16 => i16::from_le_bytes([ch[0], ch[1]]) as f32 / 32768.0,
                _ => unreachable!(),
            };
            samples.push(normalized);
        }
    }

    Ok(WavData {
        info: WavInfo { sample_rate, channels, bits_per_sample, num_samples_per_channel: num_frames as u32 },
        samples,
    })
}

/// 16bit PCM WAVとして書き出す(出力ビット深度は固定で16bitに統一)。
pub fn write(path: &Path, sample_rate: u32, channels: u16, samples: &[f32]) -> io::Result<()> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_bytes = samples.len() * 2;

    let mut out = Vec::with_capacity(44 + data_bytes);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let value = (clamped * 32767.0) as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }

    fs::write(path, out)
}

/// 単純な線形補間によるリサンプル(高品質なフィルタは無し、v0.1の割り切り)。
pub fn resample(data: &WavData, target_rate: u32) -> Vec<f32> {
    if target_rate == data.info.sample_rate || data.info.sample_rate == 0 {
        return data.samples.clone();
    }
    let channels = data.info.channels as usize;
    let src_frames = data.info.num_samples_per_channel as usize;
    let ratio = data.info.sample_rate as f64 / target_rate as f64;
    let dst_frames = ((src_frames as f64) / ratio).round() as usize;

    let mut out = Vec::with_capacity(dst_frames * channels);
    for i in 0..dst_frames {
        let src_pos = i as f64 * ratio;
        let idx0 = src_pos.floor() as usize;
        let idx1 = (idx0 + 1).min(src_frames.saturating_sub(1));
        let frac = (src_pos - idx0 as f64) as f32;
        for ch in 0..channels {
            let a = data.samples.get(idx0 * channels + ch).copied().unwrap_or(0.0);
            let b = data.samples.get(idx1 * channels + ch).copied().unwrap_or(0.0);
            out.push(a + (b - a) * frac);
        }
    }
    out
}

/// ステレオ<->モノラルの変換(2ch以外は変換不可としてエラーを返す)。
pub fn convert_channels(data: &WavData, target_channels: u16) -> io::Result<Vec<f32>> {
    let src_channels = data.info.channels;
    if src_channels == target_channels {
        return Ok(data.samples.clone());
    }
    let frames = data.info.num_samples_per_channel as usize;

    match (src_channels, target_channels) {
        (2, 1) => Ok((0..frames)
            .map(|i| (data.samples[i * 2] + data.samples[i * 2 + 1]) / 2.0)
            .collect()),
        (1, 2) => Ok((0..frames).flat_map(|i| [data.samples[i], data.samples[i]]).collect()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{src_channels}ch -> {target_channels}ch の変換はv0.1では未対応です / unsupported channel conversion"),
        )),
    }
}

#[allow(dead_code)]
fn _unused(_r: impl Read) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_test_wav(path: &Path, sample_rate: u32, channels: u16, frames: &[i16]) {
        let mut f = fs::File::create(path).unwrap();
        let data_bytes = frames.len() * 2;
        f.write_all(b"RIFF").unwrap();
        f.write_all(&((36 + data_bytes) as u32).to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap();
        f.write_all(&channels.to_le_bytes()).unwrap();
        f.write_all(&sample_rate.to_le_bytes()).unwrap();
        let byte_rate = sample_rate * channels as u32 * 2;
        f.write_all(&byte_rate.to_le_bytes()).unwrap();
        f.write_all(&(channels * 2).to_le_bytes()).unwrap();
        f.write_all(&16u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&(data_bytes as u32).to_le_bytes()).unwrap();
        for &s in frames {
            f.write_all(&s.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn reads_back_pcm16_mono_correctly() {
        let path = std::env::temp_dir().join("rs_ffmpeg_test_mono.wav");
        write_test_wav(&path, 8000, 1, &[0, 16384, -16384, 32767]);
        let data = read(&path).unwrap();
        assert_eq!(data.info.sample_rate, 8000);
        assert_eq!(data.info.channels, 1);
        assert_eq!(data.info.num_samples_per_channel, 4);
        assert!((data.samples[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn write_then_read_roundtrips_within_quantization_error() {
        let path = std::env::temp_dir().join("rs_ffmpeg_test_roundtrip.wav");
        let original = vec![0.0, 0.5, -0.5, 0.99];
        write(&path, 44100, 1, &original).unwrap();
        let data = read(&path).unwrap();
        for (a, b) in original.iter().zip(data.samples.iter()) {
            assert!((a - b).abs() < 0.001, "{a} vs {b}");
        }
    }

    #[test]
    fn resample_changes_frame_count_proportionally() {
        let path = std::env::temp_dir().join("rs_ffmpeg_test_resample.wav");
        let frames: Vec<i16> = (0..1000).map(|i| (i % 100) as i16 * 100).collect();
        write_test_wav(&path, 8000, 1, &frames);
        let data = read(&path).unwrap();
        let resampled = resample(&data, 16000);
        assert!((resampled.len() as i64 - 2000).abs() < 5);
    }

    #[test]
    fn stereo_to_mono_averages_channels() {
        let path = std::env::temp_dir().join("rs_ffmpeg_test_stereo.wav");
        write_test_wav(&path, 8000, 2, &[10000, -10000, 0, 0]);
        let data = read(&path).unwrap();
        let mono = convert_channels(&data, 1).unwrap();
        assert!((mono[0]).abs() < 0.001);
    }
}
