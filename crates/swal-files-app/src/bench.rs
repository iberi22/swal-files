#![forbid(unsafe_code)]

//! Headless CLI Benchmark & Scan Throughput Test for SWAL Files (`swal-files-app`).
//! Evaluates directory scanning throughput (up to 100k files) and verifies UI frame budget metrics.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};
use swal_files_core::scanner::{DirectoryScanner, FileEntry, ScanOptions};
use tempfile::TempDir;

/// Configuration and executor for headless file scanning throughput and frame budget benchmarks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanBenchmark {
    /// Target number of synthetic files to generate and scan
    pub file_count: usize,
    /// Maximum directory tree depth for file distribution
    pub dir_depth: usize,
    /// Target frame budget in milliseconds (e.g., 5.0ms for 200Hz+ engine)
    pub target_frame_budget_ms: f64,
    /// Processing batch size for frame budget chunking metrics
    pub batch_size: usize,
}

impl Default for ScanBenchmark {
    fn default() -> Self {
        Self {
            file_count: 10_000,
            dir_depth: 3,
            target_frame_budget_ms: 5.0,
            batch_size: 1_000,
        }
    }
}

impl ScanBenchmark {
    /// Constructs a new [`ScanBenchmark`] initialized with the given target file count.
    pub fn new(file_count: usize) -> Self {
        Self { file_count, ..Default::default() }
    }

    /// Configures the directory recursion depth for synthetic testing.
    pub fn with_dir_depth(mut self, depth: usize) -> Self {
        self.dir_depth = depth;
        self
    }

    /// Configures the target frame budget threshold in milliseconds.
    pub fn with_frame_budget_ms(mut self, budget_ms: f64) -> Self {
        self.target_frame_budget_ms = budget_ms;
        self
    }

    /// Configures the batch size used for calculating chunked frame latency.
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Runs the benchmark scan against an existing target path on the file system.
    pub async fn run_on_path<P: AsRef<Path>>(&self, path: P) -> std::io::Result<BenchmarkResult> {
        let path = path.as_ref();
        let scanner = DirectoryScanner::new(ScanOptions::default().with_recursive(true));

        let start = Instant::now();
        let entries: Vec<FileEntry> = scanner.scan(path).await?;
        let elapsed = start.elapsed();

        let total_files = entries.iter().filter(|e| e.file_type.is_file()).count();
        let total_directories = entries.iter().filter(|e| e.is_dir()).count();
        let total_bytes: u64 = entries.iter().map(|e| e.size).sum();

        let elapsed_secs = elapsed.as_secs_f64();
        let files_per_second = if elapsed_secs > 0.0 { entries.len() as f64 / elapsed_secs } else { 0.0 };
        let mb_per_second = if elapsed_secs > 0.0 { (total_bytes as f64 / (1024.0 * 1024.0)) / elapsed_secs } else { 0.0 };

        let batch_count = (entries.len() + self.batch_size - 1) / self.batch_size.max(1);
        let avg_batch_latency_ms = if batch_count > 0 { (elapsed_secs * 1000.0) / batch_count as f64 } else { 0.0 };
        let frame_budget_exceeded = avg_batch_latency_ms > self.target_frame_budget_ms;

        Ok(BenchmarkResult {
            total_files,
            total_directories,
            total_bytes,
            elapsed,
            files_per_second,
            mb_per_second,
            frame_budget_ms: self.target_frame_budget_ms,
            average_batch_latency_ms: avg_batch_latency_ms,
            frame_budget_exceeded,
        })
    }

    /// Generates a synthetic directory tree and runs the throughput benchmark.
    pub async fn run_synthetic(&self) -> std::io::Result<BenchmarkResult> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        let mut dirs = vec![root.to_path_buf()];
        for i in 0..self.dir_depth {
            let sub = root.join(format!("sub_{}", i));
            tokio::fs::create_dir_all(&sub).await?;
            dirs.push(sub);
        }

        let sample_data = b"SWAL Files Headless Benchmark Test File Content Buffer Data";
        for i in 0..self.file_count {
            let target_dir = &dirs[i % dirs.len()];
            let file_path = target_dir.join(format!("bench_{}.txt", i));
            tokio::fs::write(&file_path, sample_data).await?;
        }

        self.run_on_path(root).await
    }
}

/// Execution output and performance metrics resulting from a [`ScanBenchmark`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Count of regular files scanned
    pub total_files: usize,
    /// Count of directories scanned
    pub total_directories: usize,
    /// Total bytes of metadata processed
    pub total_bytes: u64,
    /// Total duration elapsed during scanning
    pub elapsed: Duration,
    /// Throughput measured in files scanned per second
    pub files_per_second: f64,
    /// Data throughput measured in megabytes per second
    pub mb_per_second: f64,
    /// Target UI frame budget in milliseconds
    pub frame_budget_ms: f64,
    /// Calculated average latency per batch in milliseconds
    pub average_batch_latency_ms: f64,
    /// True if average batch latency exceeded the target frame budget
    pub frame_budget_exceeded: bool,
}

impl BenchmarkResult {
    /// Returns `true` if the benchmark completed without exceeding the target frame budget.
    pub fn is_success(&self) -> bool {
        !self.frame_budget_exceeded
    }

    /// Returns a formatted single-line performance summary string.
    pub fn summary(&self) -> String {
        format!(
            "Scanned {} files / {} dirs in {:.2?} ({:.0} files/s, {:.2} MB/s) | Batch: {:.3}ms / Budget: {:.1}ms [{}]",
            self.total_files,
            self.total_directories,
            self.elapsed,
            self.files_per_second,
            self.mb_per_second,
            self.average_batch_latency_ms,
            self.frame_budget_ms,
            if self.is_success() { "PASS" } else { "EXCEEDED" }
        )
    }
}

/// Asynchronously runs a synthetic scan benchmark with default configuration for `file_count`.
pub async fn run_synthetic_benchmark(file_count: usize) -> std::io::Result<BenchmarkResult> {
    ScanBenchmark::new(file_count).run_synthetic().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_builder_defaults() {
        let bench = ScanBenchmark::new(500)
            .with_dir_depth(2)
            .with_frame_budget_ms(4.0)
            .with_batch_size(100);

        assert_eq!(bench.file_count, 500);
        assert_eq!(bench.dir_depth, 2);
        assert_eq!(bench.target_frame_budget_ms, 4.0);
        assert_eq!(bench.batch_size, 100);
    }

    #[tokio::test]
    async fn test_run_synthetic_benchmark_small() {
        let res = run_synthetic_benchmark(20).await;
        assert!(res.is_ok());
        let result = res.unwrap();
        assert_eq!(result.total_files, 20);
        assert!(result.total_directories >= 1);
        assert!(result.total_bytes > 0);
        assert!(!result.summary().is_empty());
    }

    #[tokio::test]
    async fn test_frame_budget_metrics() {
        let bench = ScanBenchmark::new(10)
            .with_frame_budget_ms(1000.0)
            .with_batch_size(5);
        let res = bench.run_synthetic().await.unwrap();
        assert!(res.is_success());
        assert!(!res.frame_budget_exceeded);
        assert!(res.summary().contains("PASS"));
    }
}
