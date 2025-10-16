use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Clone)]
pub struct Determinants {
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Version {
    pub name: String,
    pub update_link: Option<String>,
    pub jumpstart_link: Option<String>,
    pub determinants: Vec<Determinants>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Source {
    pub versions: Vec<Version>,
}

impl Source {
    pub fn get_current_version(&self, path: &Path) -> Result<Option<usize>, std::io::Error> {
        let mut already_calculated: HashMap<&str, [u8; 64]> = HashMap::new();
        'version: for (i, version) in self.versions.iter().enumerate() {
            for determinant in &version.determinants {
                let hash = if let Some(x) = already_calculated.get(determinant.file.as_str()) {
                    x
                } else {
                    let mut hasher = Sha256::new();
                    std::io::copy(
                        &mut BufReader::new(File::open(path.join(&determinant.file))?),
                        &mut hasher,
                    )?;
                    let mut buffer = [0; 64];
                    base16ct::lower::encode(&hasher.finalize(), &mut buffer).unwrap();
                    already_calculated.insert(&determinant.file, buffer);
                    already_calculated.get(&determinant.file.as_str()).unwrap()
                };

                // This can panic if the hash is badly formatted
                if hash != determinant.sha256.as_bytes() {
                    continue 'version;
                }
            }

            return Ok(Some(i));
        }
        Ok(None)
    }
}
