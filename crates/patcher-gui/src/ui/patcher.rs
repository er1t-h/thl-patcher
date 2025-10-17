use std::{
    io::{self, Cursor},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

use crate::structures::{config::PatcherConfig, source::Source};
use eframe::egui::{Color32, ProgressBar, Ui};
use tempfile::tempdir;
use thiserror::Error;
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Debug)]
enum Version {
    NotFetched,
    NotFound,
    Found(usize),
}

pub struct Patcher {
    source: Source,
    progress: Progress,
    version: Version,
    selected_path: Option<String>,
    receiver: Option<Receiver<Action>>,
}
pub enum Progress {
    NotUpdating,
    Updating { versions: f32, files: f32 },
    Updated,
}

pub enum Action {
    UpdateProgress(Progress),
    UpVersion,
    FinishUpdate,
}

#[derive(Error, Debug)]
pub enum GetVersionError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("version not found")]
    VersionNotFound,
    #[error("missing path")]
    MissingPath,
}

impl Patcher {
    fn refresh_current_version(&mut self) {
        let version = (|| {
            let path = self
                .selected_path
                .as_ref()
                .ok_or(GetVersionError::MissingPath)?;
            let version = self
                .source
                .get_current_version(&Path::new(path))?
                .ok_or(GetVersionError::VersionNotFound)?;
            Ok(version)
        })();
        self.version = match version {
            Ok(x) => Version::Found(x),
            Err(GetVersionError::VersionNotFound) => Version::NotFound,
            Err(GetVersionError::MissingPath) => Version::NotFetched,
            Err(GetVersionError::Io(x)) => panic!("{x}")
        };
    }

    pub fn new(config: PatcherConfig, source: Source) -> Self {
        let mut patcher = Self {
            source,
            version: Version::NotFetched,
            progress: Progress::NotUpdating,
            selected_path: config.get_default_path(),
            receiver: None,
        };
        if patcher.selected_path.is_some() {
            patcher.refresh_current_version();
        }
        patcher
    }

    pub fn update(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Gestionnaire de Mise à Jour");

            match self.version {
                Version::NotFetched => (),
                Version::NotFound => {
                    ui.colored_label(Color32::RED, "Votre version n'a pas ete trouvee.");
                    ui.colored_label(Color32::RED, "Tentez de verifier l'integrite des fichiers.");
                    ui.colored_label(
                        Color32::RED,
                        "Si cela ne fonctionne pas, attendez une mise a jour.",
                    );
                    if ui.button("Reverifier").clicked() {
                        self.refresh_current_version();
                    } 
                }
                Version::Found(version) => {
                    ui.label(format!(
                        "Version actuelle : {}",
                        &self.source.versions[version].name
                    ));
                    if self.source.versions.len() - 1 == version {
                        ui.label("Vous avez la dernière version !");
                    }
                }
            };

            ui.add_space(20.);

            ui.label("Veuillez sélectionner le dossier du jeu");
            if ui.button("Parcourir les fichiers...").clicked()
                && let Some(path) = rfd::FileDialog::new().pick_folder()
            {
                self.progress = Progress::NotUpdating;
                self.selected_path = Some(path.display().to_string());
                let _ = self.refresh_current_version();
            }
            if let Some(old) = &self.selected_path {
                ui.label("Dossier séléctionné : ");
                ui.monospace(old);
            }

            if let Some(ref old) = self.selected_path
                && let Version::Found(current_version) = self.version
            {
                if ui.button("Appliquer le Patch").clicked() {
                    let (_, new) = self
                        .source
                        .versions
                        .split_at_checked(current_version + 1)
                        .unwrap_or_default();
                    let new = new.to_vec();
                    let number_of_patch = new.len() as f32;
                    let old = old.clone();
                    let (tx, rx) = mpsc::channel();
                    self.receiver = Some(rx);
                    std::thread::spawn(move || {
                        for (i, version) in new.into_iter().enumerate() {
                            let versions = i as f32 / number_of_patch;
                            let _ = tx.send(Action::UpdateProgress(Progress::Updating {
                                versions,
                                files: 0.,
                            }));
                            // A temporary directory where patched files go
                            let temp_dir = tempdir().unwrap();
                            // A temporary directory to extract the archive
                            let patch = tempdir().unwrap();

                            let update_link = version.update_link.as_ref().unwrap();
                            let archive_content =
                                minreq::get(update_link).send().unwrap().into_bytes();
                            let mut archive =
                                ZipArchive::new(Cursor::new(archive_content)).unwrap();
                            archive.extract(patch.path()).unwrap();
                            let old = PathBuf::from(&old);

                            thl_patcher::patch(&old, &patch.path(), &temp_dir.path(), |s| {
                                let _ = tx.send(Action::UpdateProgress(Progress::Updating {
                                    versions,
                                    files: s.done as f32 / s.out_of as f32,
                                }));
                            });

                            for file in WalkDir::new(temp_dir.path()) {
                                let file = file.unwrap();
                                if !file.file_type().is_file() {
                                    continue;
                                }
                                let path = file.into_path();
                                let suffix = path.strip_prefix(temp_dir.path()).unwrap();
                                std::fs::copy(&path, old.join(suffix)).unwrap();
                            }
                            let _ = tx.send(Action::UpVersion);
                        }
                        let _ = tx.send(Action::FinishUpdate);
                    });
                }
            }

            match self.progress {
                Progress::Updating { files, versions } => {
                    ui.add(ProgressBar::new(versions).show_percentage());
                    ui.add(ProgressBar::new(files).show_percentage());
                }
                Progress::NotUpdating => (),
                Progress::Updated => {
                    ui.label("Mise à jour complétée avec succès");
                }
            }

            if let Some(rx) = &mut self.receiver {
                while let Ok(action) = rx.try_recv() {
                    match action {
                        Action::UpdateProgress(x) => self.progress = x,
                        Action::UpVersion => {
                            if let Version::Found(ref mut v) = self.version {
                                *v += 1
                            }
                        }
                        Action::FinishUpdate => self.progress = Progress::Updated,
                    }
                }
            }
        });
    }
}
