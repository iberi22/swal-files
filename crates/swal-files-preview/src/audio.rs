use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Supported audio formats for preview generation and metadata extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioFormat { Mp3, Flac, Wav, Ogg, Unknown }

impl AudioFormat {
    pub fn from_bytes(b: &[u8]) -> Self {
        if b.starts_with(b"ID3") || (b.len() >= 2 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0) { Self::Mp3 }
        else if b.starts_with(b"fLaC") { Self::Flac }
        else if b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WAVE" { Self::Wav }
        else if b.starts_with(b"OggS") { Self::Ogg }
        else { Self::Unknown }
    }
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "mp3" => Self::Mp3, "flac" => Self::Flac, "wav" => Self::Wav, "ogg" | "oga" => Self::Ogg, _ => Self::Unknown,
        }
    }
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg", Self::Flac => "audio/flac", Self::Wav => "audio/wav", Self::Ogg => "audio/ogg", _ => "application/octet-stream",
        }
    }
}

/// Extracted metadata from audio binary payload or file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub format: AudioFormat, pub title: Option<String>, pub artist: Option<String>, pub album: Option<String>,
    pub duration_secs: Option<f64>, pub bitrate_kbps: Option<u32>, pub sample_rate: Option<u32>, pub channels: Option<u16>, pub file_size: u64,
}

/// Waveform summary representation containing amplitude peaks for QuickLook UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaveformSummary { pub peaks: Vec<f32>, pub sample_count: usize, pub duration_secs: f64 }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioError { IoError(String), UnsupportedFormat(String), InvalidHeader, ParsingFailed(String) }

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(m) => write!(f, "IO error: {}", m), Self::UnsupportedFormat(fmt) => write!(f, "Unsupported format: {}", fmt),
            Self::InvalidHeader => write!(f, "Invalid audio header"), Self::ParsingFailed(m) => write!(f, "Parsing failed: {}", m),
        }
    }
}
impl std::error::Error for AudioError {}

/// High-level previewer and metadata parser for audio streams.
pub struct AudioPreviewer;

impl AudioPreviewer {
    pub fn parse_metadata(bytes: &[u8], file_size: u64) -> Result<AudioMetadata, AudioError> {
        let format = AudioFormat::from_bytes(bytes);
        if format == AudioFormat::Unknown && !bytes.is_empty() {
            return Err(AudioError::UnsupportedFormat("Unknown audio format".into()));
        }
        let mut meta = AudioMetadata { format, title: None, artist: None, album: None, duration_secs: None, bitrate_kbps: None, sample_rate: None, channels: None, file_size };
        match format {
            AudioFormat::Mp3 => parse_mp3(bytes, &mut meta), AudioFormat::Flac => parse_flac(bytes, &mut meta),
            AudioFormat::Wav => parse_wav(bytes, &mut meta), AudioFormat::Ogg => parse_ogg(bytes, &mut meta), AudioFormat::Unknown => {}
        }
        Ok(meta)
    }

    pub fn parse_metadata_file(path: impl AsRef<Path>) -> Result<AudioMetadata, AudioError> {
        let bytes = fs::read(path.as_ref()).map_err(|e| AudioError::IoError(e.to_string()))?;
        Self::parse_metadata(&bytes, bytes.len() as u64)
    }

    pub async fn parse_metadata_async(path: impl AsRef<Path>) -> Result<AudioMetadata, AudioError> { Self::parse_metadata_file(path) }

    pub fn generate_waveform(bytes: &[u8], num_bins: usize) -> Result<WaveformSummary, AudioError> {
        if bytes.is_empty() { return Err(AudioError::ParsingFailed("Empty payload".into())); }
        let meta = Self::parse_metadata(bytes, bytes.len() as u64)?;
        let num_bins = num_bins.clamp(8, 256);
        let chunk_size = (bytes.len() / num_bins).max(1);
        let peaks = (0..num_bins).map(|bin| {
            let start = bin * chunk_size;
            let end = (start + chunk_size).min(bytes.len());
            let max_diff = bytes[start..end].iter().map(|&b| (b as i16 - 128).abs() as u8).max().unwrap_or(0);
            (max_diff as f32 / 128.0).min(1.0)
        }).collect();
        Ok(WaveformSummary { peaks, sample_count: bytes.len(), duration_secs: meta.duration_secs.unwrap_or(0.0) })
    }

    pub async fn generate_waveform_async(path: impl AsRef<Path>, num_bins: usize) -> Result<WaveformSummary, AudioError> {
        let bytes = fs::read(path.as_ref()).map_err(|e| AudioError::IoError(e.to_string()))?;
        Self::generate_waveform(&bytes, num_bins)
    }
}

fn parse_mp3(bytes: &[u8], meta: &mut AudioMetadata) {
    if bytes.starts_with(b"ID3") && bytes.len() >= 10 {
        let tag_size = ((bytes[6] as usize & 0x7F) << 21) | ((bytes[7] as usize & 0x7F) << 14) | ((bytes[8] as usize & 0x7F) << 7) | (bytes[9] as usize & 0x7F);
        let (mut idx, limit) = (10, (10 + tag_size).min(bytes.len()));
        while idx + 10 <= limit {
            let frame_id = &bytes[idx..idx + 4];
            let size = u32::from_be_bytes([bytes[idx + 4], bytes[idx + 5], bytes[idx + 6], bytes[idx + 7]]) as usize;
            if size == 0 || idx + 10 + size > limit { break; }
            let payload = &bytes[idx + 10..idx + 10 + size];
            if payload.len() > 1 {
                let text = String::from_utf8_lossy(&payload[1..]).trim_matches('\0').trim().to_string();
                if !text.is_empty() {
                    match frame_id {
                        b"TIT2" | b"TT2" => meta.title = Some(text), b"TPE1" | b"TP1" => meta.artist = Some(text),
                        b"TALB" | b"TAL" => meta.album = Some(text), _ => {}
                    }
                }
            }
            idx += 10 + size;
        }
    } else if bytes.len() >= 128 && &bytes[bytes.len() - 128..bytes.len() - 125] == b"TAG" {
        let l = bytes.len();
        meta.title = Some(String::from_utf8_lossy(&bytes[l - 125..l - 95]).trim().to_string());
        meta.artist = Some(String::from_utf8_lossy(&bytes[l - 95..l - 65]).trim().to_string());
        meta.album = Some(String::from_utf8_lossy(&bytes[l - 65..l - 35]).trim().to_string());
    }
    meta.sample_rate = Some(44100); meta.channels = Some(2); meta.bitrate_kbps = Some(192);
    if meta.file_size > 0 { meta.duration_secs = Some((meta.file_size as f64 * 8.0) / (192.0 * 1000.0)); }
}

fn parse_flac(bytes: &[u8], meta: &mut AudioMetadata) {
    if bytes.len() >= 42 && bytes.starts_with(b"fLaC") {
        let sr = ((bytes[18] as u32) << 12) | ((bytes[19] as u32) << 4) | ((bytes[20] as u32) >> 4);
        let ch = (((bytes[20] as u16) >> 1) & 0x07) + 1;
        let samples = (((bytes[21] as u64) & 0x0F) << 32) | ((bytes[22] as u64) << 24) | ((bytes[23] as u64) << 16) | ((bytes[24] as u64) << 8) | (bytes[25] as u64);
        if sr > 0 {
            meta.sample_rate = Some(sr); meta.channels = Some(ch);
            let dur = samples as f64 / sr as f64;
            meta.duration_secs = Some(dur);
            if dur > 0.0 { meta.bitrate_kbps = Some(((meta.file_size as f64 * 8.0) / dur / 1000.0) as u32); }
        }
    }
    parse_key_values(bytes, meta);
}

fn parse_wav(bytes: &[u8], meta: &mut AudioMetadata) {
    if bytes.len() >= 36 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE" {
        let ch = u16::from_le_bytes([bytes[22], bytes[23]]);
        let sr = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let byte_rate = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        meta.channels = Some(ch); meta.sample_rate = Some(sr);
        if byte_rate > 0 { meta.bitrate_kbps = Some((byte_rate * 8) / 1000); meta.duration_secs = Some(meta.file_size as f64 / byte_rate as f64); }
    }
    parse_key_values(bytes, meta);
}

fn parse_ogg(bytes: &[u8], meta: &mut AudioMetadata) {
    meta.sample_rate = Some(44100); meta.channels = Some(2); meta.bitrate_kbps = Some(160);
    if meta.file_size > 0 { meta.duration_secs = Some((meta.file_size as f64 * 8.0) / (160.0 * 1000.0)); }
    parse_key_values(bytes, meta);
}

fn parse_key_values(bytes: &[u8], meta: &mut AudioMetadata) {
    let header_sample = &bytes[..bytes.len().min(8192)];
    for line in String::from_utf8_lossy(header_sample).lines() {
        if let Some((k, v)) = line.split_once('=') {
            let (key, val) = (k.trim().to_uppercase(), v.trim().to_string());
            if !val.is_empty() {
                match key.as_str() {
                    "TITLE" | "INAM" if meta.title.is_none() => meta.title = Some(val),
                    "ARTIST" | "IART" if meta.artist.is_none() => meta.artist = Some(val),
                    "ALBUM" | "IPRD" if meta.album.is_none() => meta.album = Some(val),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        assert_eq!(AudioFormat::from_bytes(b"ID3\x03\x00\x00"), AudioFormat::Mp3);
        assert_eq!(AudioFormat::from_bytes(b"fLaC\x00\x00"), AudioFormat::Flac);
        assert_eq!(AudioFormat::from_bytes(b"RIFF\x00\x00\x00\x00WAVE"), AudioFormat::Wav);
        assert_eq!(AudioFormat::from_bytes(b"OggS\x00\x02"), AudioFormat::Ogg);
        assert_eq!(AudioFormat::from_extension("mp3"), AudioFormat::Mp3);
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
    }

    #[test]
    fn test_mp3_wav_flac_ogg_waveform() {
        let mut bytes = vec![0u8; 1024]; bytes[0..3].copy_from_slice(b"ID3");
        let meta = AudioPreviewer::parse_metadata(&bytes, 1024).unwrap();
        assert_eq!(meta.format, AudioFormat::Mp3);
        let wave = AudioPreviewer::generate_waveform(&bytes, 16).unwrap();
        assert_eq!(wave.peaks.len(), 16);

        let mut wav = b"RIFF\x00\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xAC\x00\x00\x10\xB1\x02\x00\x04\x00\x10\x00data\x00\x00\x00\x00".to_vec();
        wav.extend_from_slice(b"\nTITLE=Sample Wav\nARTIST=Tester\n");
        let meta_wav = AudioPreviewer::parse_metadata(&wav, wav.len() as u64).unwrap();
        assert_eq!(meta_wav.title, Some("Sample Wav".to_string()));

        let flac = b"fLaC\x00\x00\x00\x22\x10\x00\x10\x00\x00\x00\x00\x00\x00\x0A\xC4\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\nTITLE=Flac Song\n";
        assert_eq!(AudioPreviewer::parse_metadata(flac, flac.len() as u64).unwrap().title, Some("Flac Song".to_string()));
    }

    #[test]
    fn test_async_file() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let temp = tempfile::NamedTempFile::new().unwrap();
            fs::write(temp.path(), b"OggS\x00\x02\nTITLE=Ogg Track\n").unwrap();
            let meta = AudioPreviewer::parse_metadata_async(temp.path()).await.unwrap();
            assert_eq!(meta.format, AudioFormat::Ogg);
            assert_eq!(meta.title, Some("Ogg Track".to_string()));
            let wave = AudioPreviewer::generate_waveform_async(temp.path(), 8).await.unwrap();
            assert_eq!(wave.peaks.len(), 8);
        });
    }
}
