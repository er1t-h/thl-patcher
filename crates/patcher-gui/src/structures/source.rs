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
    pub determinants: Vec<Determinants>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Source {
    pub versions: Vec<Version>,
}

impl Source {
    pub fn get_current_version(&self, path: &Path) -> Result<Option<usize>, std::io::Error> {
        let mut already_calculated: HashMap<&str, [u8; 64]> = HashMap::new();
        'version: for (i, version) in self.versions.iter().enumerate().rev() {
            tracing::trace!("checking version `{}`", version.name);
            for determinant in &version.determinants {
                let hash = if let Some(x) = already_calculated.get(determinant.file.as_str()) {
                    x
                } else {
                    let mut hasher = Sha256::new();
                    let path = path.join(&determinant.file);
                    let file = match File::open(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            tracing::trace!("error while opening file `{}`: `{e}`", determinant.file);
                            continue 'version
                        }
                    };
                    std::io::copy(
                        &mut BufReader::new(file),
                        &mut hasher,
                    )?;
                    let mut buffer = [0; 64];
                    match base16ct::lower::encode(&hasher.finalize(), &mut buffer) {
                        Ok(_) => (),
                        Err(e) => unreachable!("64-byte should always be enough: {e}"),
                    }
                    already_calculated.entry(&determinant.file).or_insert(buffer)
                };

                if hash != determinant.sha256.as_bytes() {
                    tracing::trace!("sha256 mismatch for file `{}`", determinant.file);
                    continue 'version;
                }
            }

            return Ok(Some(i));
        }
        Ok(None)
    }

    pub fn get_versions_to_install(&self, current: usize) -> &[Version] {
        let versions_to_install = self
            .versions
            .split_at_checked(current)
            .unwrap_or_default()
            .1;

        versions_to_install
            .iter()
            .position(|x| x.update_link.is_none())
            .map_or(versions_to_install, |pos| versions_to_install.split_at(pos).0)
    }
}
