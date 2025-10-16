use std::{
    env::current_dir, io::Cursor, path::{Path, PathBuf}
};

use eframe::egui;
use tempfile::tempdir;
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::structures::{config::PatcherConfig, source::Source};

mod error;
mod structures;

fn get_config() -> PatcherConfig {
    if let Ok(file) = std::fs::read_to_string("config.yaml")
        && let Ok(config) = serde_yaml::from_str(&file)
    {
        config
    } else {
        PatcherConfig::default()
    }
}

fn main() {
    println!("Patcher réalisé par Er1t : https://github.com/er1t-h/thl-patcher");
    let config = get_config();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        &config.window_name,
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(&config)))),
    )
    .unwrap();
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    NotApplied,
    Progressing(f32),
    Applied,
}

struct MyApp {
    old: Option<String>,
    version: Option<String>,
    source: Source,
    applied: bool,
}

impl MyApp {
    fn new(config: &PatcherConfig) -> Self {
        let source =
            serde_yaml::from_slice(&minreq::get(&config.source).send().unwrap().as_bytes()).unwrap();
        Self {
            old: config.get_default_path(),
            source,
            version: None,
            applied: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Sélectionnez le dossier du jeu");
                if ui.button("Parcourir les fichiers...").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.applied = false;
                    self.old = Some(path.display().to_string());
                }
                if let Some(old) = &self.old {
                    ui.label("Dossier séléctionné : ");
                    ui.monospace(old);
                }

                if let Some(ref old) = self.old {
                    if let Some(x) = &self.version {
                        ui.label(x);
                    }
                    if ui.button("Appliquer le Patch").clicked() {
                        let current_version = self
                            .source
                            .get_current_version(Path::new(old))
                            .unwrap()
                            .unwrap();
                        let (_, new) = self
                            .source
                            .versions
                            .split_at_checked(current_version + 1)
                            .unwrap_or_default();
                        for version in new {
                            // A temporary directory where patched files go
                            let temp_dir = tempdir().unwrap();
                            // A temporary directory to extract the archive
                            let patch = tempdir().unwrap();

                            let update_link = version.update_link.as_ref().unwrap();
                            let archive_content = minreq::get(update_link).send().unwrap().into_bytes();
                            let mut archive = ZipArchive::new(Cursor::new(archive_content)).unwrap();
                            archive.extract(patch.path()).unwrap();
                            let old = PathBuf::from(old);

                            thl_patcher::patch(
                                &old,
                                &patch.path(),
                                &temp_dir.path(),
                                |_| (),
                            );

                            for file in WalkDir::new(temp_dir.path()) {
                                let file = file.unwrap();
                                if !file.file_type().is_file() {
                                    continue;
                                }
                                let path = file.into_path();
                                let suffix = path.strip_prefix(temp_dir.path()).unwrap();
                                std::fs::copy(&path, old.join(suffix)).unwrap();
                            }
                        }

                        self.old = None;
                    }
                }

                if self.applied {
                    ui.label("Patch appliqué avec succès !");
                }
            })
        });
    }
}
