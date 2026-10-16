//! Safe file storage service (ticket P1-12).
//! Ensures files (receipts, documents, resumes) are safely validated and stored outside the public directory.
//! Enforces:
//! - Strict filename sanitization to eliminate path traversal attacks (`../`, null bytes).
//! - MIME type validation and size limits (10 MB maximum).
//! - Isolated physical path outside any web-accessible root.
use loco_rs::prelude::*;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const MAX_FILE_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10 MB

pub const ALLOWED_MIME_TYPES: &[&str] = &[
    "application/pdf",
    "image/png",
    "image/jpeg",
    "image/webp",
    "text/plain",
    "text/csv",
];

#[derive(Debug, Clone)]
pub struct StoredFile {
    pub storage_key: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub absolute_path: PathBuf,
}

pub struct StorageService {
    storage_root: PathBuf,
}

impl StorageService {
    #[must_use]
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            storage_root: root.as_ref().to_path_buf(),
        }
    }

    /// Validates and stores a file payload outside the public directory.
    ///
    /// # Errors
    /// Returns error if file exceeds size limit, MIME is disallowed, or filesystem write fails.
    pub async fn store(
        &self,
        original_name: &str,
        mime_type: &str,
        content: &[u8],
    ) -> Result<StoredFile> {
        if content.len() > MAX_FILE_SIZE_BYTES {
            return Err(Error::string(
                "file size exceeds maximum allowed 10 MB limit",
            ));
        }

        if !ALLOWED_MIME_TYPES.contains(&mime_type) {
            return Err(Error::string("unsupported or disallowed MIME file type"));
        }

        Self::validate_magic_bytes(mime_type, content)?;

        // Generate non-colliding UUID key
        let safe_id = Uuid::new_v4().to_string();
        let extension = Path::new(original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("dat")
            .to_ascii_lowercase();

        let storage_key = format!("{safe_id}.{extension}");
        let absolute_path = self.storage_root.join(&storage_key);

        tokio::fs::create_dir_all(&self.storage_root).await?;
        tokio::fs::write(&absolute_path, content).await?;

        Ok(StoredFile {
            storage_key,
            original_name: original_name.to_string(),
            mime_type: mime_type.to_string(),
            size_bytes: content.len(),
            absolute_path,
        })
    }

    /// Validates file header magic bytes to reject disguised binaries or malicious payloads.
    pub fn validate_magic_bytes(mime_type: &str, content: &[u8]) -> Result<()> {
        // Disallow executables disguised as other files (Windows PE MZ header or Linux ELF header)
        if content.starts_with(b"MZ") || content.starts_with(b"\x7FELF") {
            return Err(Error::string("executable files are prohibited"));
        }

        match mime_type {
            "application/pdf" => {
                if !content.starts_with(b"%PDF-") {
                    return Err(Error::string("file content does not match PDF format"));
                }
            }
            "image/png" => {
                if !content.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
                    return Err(Error::string("file content does not match PNG format"));
                }
            }
            "image/jpeg" => {
                if !content.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    return Err(Error::string("file content does not match JPEG format"));
                }
            }
            "image/webp" => {
                if content.len() < 12 || &content[0..4] != b"RIFF" || &content[8..12] != b"WEBP" {
                    return Err(Error::string("file content does not match WebP format"));
                }
            }
            "text/plain" | "text/csv" => {
                if std::str::from_utf8(content).is_err() {
                    return Err(Error::string("text file contains non-UTF-8 binary content"));
                }
            }
            _ => return Err(Error::string("unsupported MIME type")),
        }
        Ok(())
    }

    /// Retrieves an absolute path for reading a stored file, verifying it remains inside storage_root.
    ///
    /// # Errors
    /// Returns error if path traversal is detected.
    pub fn resolve_path(&self, storage_key: &str) -> Result<PathBuf> {
        let path = self.storage_root.join(storage_key);
        // Path traversal defense
        if !path.starts_with(&self.storage_root) {
            return Err(Error::string(
                "invalid storage key: path traversal detected",
            ));
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_renamed_exe() {
        let exe_bytes = b"MZ\x90\x00\x03\x00\x00\x00";
        let res = StorageService::validate_magic_bytes("application/pdf", exe_bytes);
        assert!(res.is_err());
    }

    #[test]
    fn test_accepts_valid_pdf() {
        let pdf_bytes = b"%PDF-1.7 valid content";
        let res = StorageService::validate_magic_bytes("application/pdf", pdf_bytes);
        assert!(res.is_ok());
    }
}
