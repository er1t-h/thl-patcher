use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DefaultPaths {
    pub target_os: String,
    pub possible_paths: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PatcherConfig {
    pub window_name: String,
    pub source: String,
    pub default_paths: Vec<DefaultPaths>,
}

impl Default for PatcherConfig {
    fn default() -> Self {
        Self {
            window_name: String::from("Patcher"),
            default_paths: vec![],
            source: String::new(),
        }
    }
}

impl PatcherConfig {
    pub fn get_default_path(&self) -> Option<String> {
        for entry in self
            .default_paths
            .iter()
            .filter(|x| x.target_os == std::env::consts::OS)
        {
            for path in &entry.possible_paths {
                let path = shellexpand::tilde(path);
                let hypothesis = Path::new(path.as_ref());
                if hypothesis.exists() {
                    return Some(path.to_string());
                }
            }
        }
        None
    }
}
