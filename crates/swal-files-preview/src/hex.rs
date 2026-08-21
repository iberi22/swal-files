use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Configuration for hex chunk formatting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexChunkConfig {
    pub bytes_per_row: usize,
    pub max_bytes: usize,
    pub uppercase: bool,
}

impl Default for HexChunkConfig {
    fn default() -> Self {
        Self {
            bytes_per_row: 16,
            max_bytes: 1_000_000,
            uppercase: false,
        }
    }
}

/// Represents a single formatted row of a hex dump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HexDumpRow {
    pub offset: usize,
    pub bytes: Vec<u8>,
    pub hex_bytes: String,
    pub ascii: String,
}

impl HexDumpRow {
    /// Formats row into standard QuickLook hex line representation.
    pub fn format_line(&self, bytes_per_row: usize) -> String {
        format!(
            "{:08x}  {:width$}  |{}|",
            self.offset, self.hex_bytes, self.ascii, width = bytes_per_row * 3 - 1
        )
    }
}

/// Error type for HexInspector operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HexError {
    IoError(String),
    ExceedsLimit { size: usize, limit: usize },
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HexError::IoError(msg) => write!(f, "IO Error: {}", msg),
            HexError::ExceedsLimit { size, limit } => {
                write!(f, "Size ({} bytes) exceeds limit ({} bytes)", size, limit)
            }
        }
    }
}

impl std::error::Error for HexError {}

/// Memory-efficient QuickLook Hex & Binary File Inspector.
#[derive(Debug, Clone, Default)]
pub struct HexInspector {
    config: HexChunkConfig,
}

impl HexInspector {
    pub fn new(config: HexChunkConfig) -> Self {
        Self { config }
    }

    /// Inspects raw bytes in memory up to `max_bytes`.
    pub fn inspect_bytes(&self, data: &[u8]) -> Vec<HexDumpRow> {
        let len = data.len().min(self.config.max_bytes);
        let slice = &data[..len];
        let chunk_size = self.config.bytes_per_row.max(1);

        slice
            .chunks(chunk_size)
            .enumerate()
            .map(|(idx, chunk)| {
                let offset = idx * chunk_size;
                let hex_bytes = chunk
                    .iter()
                    .map(|b| {
                        if self.config.uppercase {
                            format!("{:02X}", b)
                        } else {
                            format!("{:02x}", b)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let ascii = chunk
                    .iter()
                    .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
                    .collect();

                HexDumpRow { offset, bytes: chunk.to_vec(), hex_bytes, ascii }
            })
            .collect()
    }

    /// Asynchronously inspects binary file content up to configured `max_bytes`.
    pub async fn inspect_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<HexDumpRow>, HexError> {
        let mut file = File::open(path.as_ref()).await.map_err(|e| HexError::IoError(e.to_string()))?;
        let mut buffer = Vec::new();
        let mut chunk = vec![0u8; 8192];
        let mut total_read = 0;

        while total_read < self.config.max_bytes {
            let to_read = (self.config.max_bytes - total_read).min(chunk.len());
            let n = file.read(&mut chunk[..to_read]).await.map_err(|e| HexError::IoError(e.to_string()))?;
            if n == 0 { break; }
            buffer.extend_from_slice(&chunk[..n]);
            total_read += n;
        }

        Ok(self.inspect_bytes(&buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_inspect_bytes_default() {
        let inspector = HexInspector::default();
        let rows = inspector.inspect_bytes(b"Hello, World!");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].offset, 0);
        assert_eq!(rows[0].ascii, "Hello, World!");
        assert_eq!(rows[0].hex_bytes, "48 65 6c 6c 6f 2c 20 57 6f 72 6c 64 21");
        let line = rows[0].format_line(16);
        assert!(line.starts_with("00000000"));
        assert!(line.contains("|Hello, World!|"));
    }

    #[test]
    fn test_custom_config_and_uppercase() {
        let config = HexChunkConfig { bytes_per_row: 8, max_bytes: 100, uppercase: true };
        let inspector = HexInspector::new(config);
        let rows = inspector.inspect_bytes(b"ABCDEFGH1234");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].hex_bytes, "41 42 43 44 45 46 47 48");
        assert_eq!(rows[1].offset, 8);
        assert_eq!(rows[1].hex_bytes, "31 32 33 34");
    }

    #[test]
    fn test_max_bytes_and_non_printable() {
        let config = HexChunkConfig { bytes_per_row: 16, max_bytes: 4, uppercase: false };
        let inspector = HexInspector::new(config);
        let rows = inspector.inspect_bytes(&[0x00, 0x07, b'A', 0xFF, 0x10, 0x20]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bytes.len(), 4);
        assert_eq!(rows[0].ascii, "..A.");
    }

    #[test]
    fn test_empty_input_and_errors() {
        let inspector = HexInspector::default();
        assert!(inspector.inspect_bytes(&[]).is_empty());
        let err1 = HexError::IoError("not found".to_string());
        assert_eq!(err1.to_string(), "IO Error: not found");
        let err2 = HexError::ExceedsLimit { size: 200, limit: 100 };
        assert_eq!(err2.to_string(), "Size (200 bytes) exceeds limit (100 bytes)");
    }

    #[tokio::test]
    async fn test_inspect_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"\x7fELFbinary_content_here").unwrap();
        let inspector = HexInspector::default();
        let rows = inspector.inspect_file(temp_file.path()).await.unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].offset, 0);
        assert!(rows[0].ascii.contains("ELFbinary"));

        let missing = inspector.inspect_file("non_existent_file_xyz_123.bin").await;
        assert!(matches!(missing, Err(HexError::IoError(_))));
    }
}
