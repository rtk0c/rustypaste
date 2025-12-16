use crate::util;
use glob::glob;
use hex::FromHexError;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File as OsFile;
use std::path::{Path, PathBuf};

/// SHA-256 digest. For use as [`HashMap`] key
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Sha256Digest(pub [u8; 32]);

impl ToString for Sha256Digest {
    fn to_string(&self) -> String {
        hex::encode(self.0)
    }
}

impl TryFrom<&str> for Sha256Digest {
    type Error = FromHexError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let b = hex::decode(value)?;
        let b: [u8; 32] = b
            .try_into()
            .or_else(|_| Err(FromHexError::InvalidStringLength))?;
        Ok(Sha256Digest(b))
    }
}

/// Index of files by their SHA-256 hash
#[derive(Debug)]
pub struct PasteIndex(pub HashMap<Sha256Digest, PathBuf>);

impl PasteIndex {
    /// Create empty content index
    pub fn new() -> Self {
        PasteIndex(HashMap::new())
    }

    /// Populate from directory content
    pub fn populate(&mut self, directory: &Path) {
        let glob_result = glob(
            // ignore None:
            // everybody yell at the glob crate author for not supporting non-UTF-8 paths
            directory.join("**").join("*").to_str().unwrap(),
        )
        // ignoring PatternError:
        // concatenating valid path with "**/*" should always produce a valid pattern
        .unwrap();

        let files_in_dir = glob_result
            .filter_map(Result::ok)
            .filter(|path| !path.is_dir())
            .filter_map(|path| {
                let file = OsFile::open(&path).ok()?;
                let sha256sum = util::sha256_digest(file).ok()?;
                Some((sha256sum, path))
            });

        for (sha256sum, path) in files_in_dir {
            self.0.insert(sha256sum, path);
        }
    }

    /// Returns the file that matches the given checksum.
    pub fn get_file(&self, sha256sum: &Sha256Digest) -> Option<&PathBuf> {
        let hash_match = self.0.get(sha256sum)?;
        if util::TIMESTAMP_EXTENSION_REGEX.is_match(&hash_match.to_string_lossy()) {
            return None;
        }
        Some(hash_match)
    }

    /// Add item to cache
    pub fn cache_file(&mut self, sha256sum: Sha256Digest, path: PathBuf) {
        self.0.insert(sha256sum, path);
    }
}

#[cfg(test)]
mod tests {
    use actix_web::Error as ActixError;

    use super::*;
    use std::ffi::OsString;

    #[test]
    fn test_file_checksum() -> Result<(), ActixError> {
        let mut index = PasteIndex::new();
        index.populate(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("img")
                .as_path(),
        );
        assert_eq!(
            Some(OsString::from("rustypaste_logo.png").as_ref()),
            index
                .get_file(
                    &Sha256Digest::try_from(
                        "78b946a10d7c2893eb76833adfe9aaff7bd8f59712653914be669928e88312cd"
                    )
                    .unwrap()
                )
                .expect("cannot get file with checksum")
                .file_name()
        );
        Ok(())
    }
}
