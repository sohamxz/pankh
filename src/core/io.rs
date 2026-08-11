use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

/// Maximum file size safety cap: 50 MB
pub const MAX_FILE_SIZE_BYTES: u64 = 50 * 1024 * 1024;

#[derive(Debug)]
pub enum SafeReadError {
    FileNotFound(String),
    FileTooLarge { path: String, size_bytes: u64 },
    BinaryFileDetected(String),
    Io(std::io::Error),
}

impl fmt::Display for SafeReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafeReadError::FileNotFound(p) => write!(f, "File not found: {}", p),
            SafeReadError::FileTooLarge { path, size_bytes } => {
                write!(
                    f,
                    "File '{}' exceeds maximum size cap of 50 MB (size: {} bytes)",
                    path, size_bytes
                )
            }
            SafeReadError::BinaryFileDetected(p) => {
                write!(f, "File '{}' appears to be a binary file", p)
            }
            SafeReadError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for SafeReadError {}

/// Safely reads a text file with 50MB size limit cap, null byte binary file detection, and UTF-8 lossy handling
pub fn read_markdown_file_safe<P: AsRef<Path>>(path: P) -> Result<String, SafeReadError> {
    let path_ref = path.as_ref();
    let path_str = path_ref.display().to_string();

    let metadata = match fs::metadata(path_ref) {
        Ok(m) => m,
        Err(_) => return Err(SafeReadError::FileNotFound(path_str)),
    };

    if metadata.len() > MAX_FILE_SIZE_BYTES {
        return Err(SafeReadError::FileTooLarge {
            path: path_str,
            size_bytes: metadata.len(),
        });
    }

    let mut file = match File::open(path_ref) {
        Ok(f) => f,
        Err(e) => return Err(SafeReadError::Io(e)),
    };

    let mut buffer = Vec::with_capacity(metadata.len() as usize);
    if let Err(e) = file.read_to_end(&mut buffer) {
        return Err(SafeReadError::Io(e));
    }

    // Inspect first 1024 bytes for null bytes (\0) to detect binary files (e.g. PDF, PNG, EXE)
    let sample_len = buffer.len().min(1024);
    if buffer[..sample_len].contains(&0) {
        return Err(SafeReadError::BinaryFileDetected(path_str));
    }

    let text = String::from_utf8_lossy(&buffer).into_owned();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_safe_read_valid_text() {
        let temp_path = std::env::temp_dir().join("pankh_test_valid.md");
        let mut file = File::create(&temp_path).unwrap();
        writeln!(file, "# Valid Markdown\nHello Pankh").unwrap();
        let text = read_markdown_file_safe(&temp_path).unwrap();
        assert!(text.contains("Valid Markdown"));
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_safe_read_binary_file() {
        let temp_path = std::env::temp_dir().join("pankh_test_binary.bin");
        let mut file = File::create(&temp_path).unwrap();
        file.write_all(&[0x00, 0x01, 0x02, 0xFF, 0x00]).unwrap();
        let err = read_markdown_file_safe(&temp_path).unwrap_err();
        assert!(matches!(err, SafeReadError::BinaryFileDetected(_)));
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_safe_read_file_not_found() {
        let err = read_markdown_file_safe("non_existent_file_xyz_123.md").unwrap_err();
        assert!(matches!(err, SafeReadError::FileNotFound(_)));
    }
}
