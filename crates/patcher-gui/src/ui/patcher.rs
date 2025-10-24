use std::{
    io::{self, Cursor},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};

use crate::structures::{config::PatcherConfig, source::Source};
use eframe::egui::{Color32, ProgressBar, RichText, Ui};
use tar::Archive;
use tempfile::tempdir;
use thiserror::Error;
use walkdir::WalkDir;
use xz2::read::XzDecoder;

#[derive(Debug)]
enum Version {
    NotFetched,
    NotFound,
    Found(usize),
    IoError(io::Error),
}

#[derive(Debug)]
enum Progress {
    NotUpdating,
    Updating {
        versions: f32,
        current: Option<PathBuf>,
    },
    Updated,
}

pub struct Patcher {
    source: Source,
    progress: Progress,
    version: Version,
    selected_path: Option<String>,
    receiver: Option<Receiver<Action>>,
    download_error: Option<DownloadAndPatchError>,
}

#[derive(Debug)]
enum Action {
    UpdateProgress(Progress),
    UpVersion,
    FinishUpdate,
    DownloadAndPatchError(DownloadAndPatchError),
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

#[derive(Error, Debug)]
pub enum DownloadAndPatchError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("io error: {0}")]
    WalkDir(#[from] walkdir::Error),
    #[error("minreq error: {0}")]
    Minreq(#[from] minreq::Error),
    #[error("patcher error: {0}")]
    PatchError(#[from] thl_patcher::PatchError),
    #[error("no update link indicated")]
    NoUpdateLink,
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
                .get_current_version(Path::new(path))?
                .ok_or(GetVersionError::VersionNotFound)?;
            Ok(version)
        })();
        self.version = match version {
            Ok(x) => Version::Found(x),
            Err(GetVersionError::VersionNotFound) => Version::NotFound,
            Err(GetVersionError::MissingPath) => Version::NotFetched,
            Err(GetVersionError::Io(err)) => Version::IoError(err),
        };
    }

    pub fn new(config: &PatcherConfig, source: Source) -> Self {
        let mut patcher = Self {
            source,
            version: Version::NotFetched,
            progress: Progress::NotUpdating,
            selected_path: config.get_default_path(),
            receiver: None,
            download_error: None,
        };
        if patcher.selected_path.is_some() {
            patcher.refresh_current_version();
        }
        patcher
    }

    fn execute_instructions_from_receiver(&mut self) {
        if let Some(rx) = &mut self.receiver {
            let mut stop_receive = false;
            while let Ok(action) = rx.try_recv() {
                match action {
                    Action::UpdateProgress(x) => self.progress = x,
                    Action::UpVersion => {
                        if let Version::Found(ref mut v) = self.version {
                            *v += 1;
                        }
                    }
                    Action::FinishUpdate => {
                        self.progress = Progress::Updated;
                        stop_receive = true;
                    }
                    Action::DownloadAndPatchError(error) => {
                        self.download_error = Some(error);
                        stop_receive = true;
                    }
                }
            }
            if stop_receive {
                self.receiver = None;
            }
        }
    }

    fn show_version(&mut self, ui: &mut Ui) {
        match self.version {
            Version::NotFetched => (),
            Version::NotFound => {
                ui.colored_label(Color32::RED, "Votre version n'a pas été trouvée.");
                ui.colored_label(Color32::RED, "Tentez de vérifier l'intégrité des fichiers.");
                ui.colored_label(
                    Color32::RED,
                    "Si cela ne fonctionne pas, attendez une mise à jour.",
                );
                if ui.button("Revérifier").clicked() {
                    self.refresh_current_version();
                }
            }
            Version::IoError(ref x) => {
                ui.colored_label(Color32::RED, "Une erreur I/O est survenue.");
                ui.colored_label(Color32::RED, "Il est probable que l'un des fichiers permettant l'évaluation de la version était absent.");
                ui.code(RichText::new(x.to_string()).color(Color32::RED));
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
        }
    }

    fn file_selector(&mut self, ui: &mut Ui) {
        ui.label("Veuillez sélectionner le dossier du jeu");
        if ui.button("Parcourir les fichiers...").clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            self.progress = Progress::NotUpdating;
            self.selected_path = Some(path.display().to_string());
            self.refresh_current_version();
        }
        if let Some(old) = &self.selected_path {
            ui.label("Dossier séléctionné : ");
            ui.monospace(old);
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn download_and_patch(
        mut transmit: impl FnMut(Action),
        original: &Path,
        new: &[crate::structures::source::Version],
    ) -> Result<(), DownloadAndPatchError> {
        let number_of_patch = new.len() as f32;

        for (i, version) in new.iter().enumerate() {
            let versions = i as f32 / number_of_patch;

            transmit(Action::UpdateProgress(Progress::Updating {
                versions,
                current: None,
            }));
            // A temporary directory where patched files go
            let temp_dir = tempdir()?;

            let update_link = version
                .update_link
                .as_ref()
                .ok_or(DownloadAndPatchError::NoUpdateLink)?;
            let archive_content = minreq::get(update_link).send()?.into_bytes();
            let decoder = XzDecoder::new(Cursor::new(archive_content));
            let mut archive = Archive::new(decoder);

            thl_patcher::patch_from_tar(original, &mut archive, temp_dir.path(), |s| {
                transmit(Action::UpdateProgress(Progress::Updating {
                    versions,
                    current: Some(s.path),
                }));
            })?;

            for file in WalkDir::new(temp_dir.path()) {
                let file = file?;
                if !file.file_type().is_file() {
                    continue;
                }
                let path = file.into_path();
                let Ok(suffix) = path.strip_prefix(temp_dir.path()) else {
                    unreachable!("path is always a child of temp_dir");
                };
                let destination = original.join(suffix);
                match std::fs::rename(&path, &destination) {
                    Ok(()) => (),
                    Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
                        std::fs::copy(&path, original.join(suffix))?;
                    }
                    Err(e) => Err(e)?,
                }
            }
            transmit(Action::UpVersion);
        }
        transmit(Action::FinishUpdate);
        Ok(())
    }

    fn apply_patch(&mut self, ui: &mut Ui) {
        if let Some(ref old) = self.selected_path
            && let Version::Found(current_version) = self.version
            && ui.button("Appliquer le Patch").clicked()
        {
            let versions_to_install = self
                .source
                .get_versions_to_install(current_version)
                .to_vec();
            let old = old.clone();
            let (tx, rx) = mpsc::channel();
            let ctx = ui.ctx().clone();
            self.receiver = Some(rx);
            std::thread::spawn(move || {
                let res = Self::download_and_patch(
                    |message| {
                        let _ = tx.send(message);
                        ctx.request_repaint();
                    },
                    Path::new(&old),
                    &versions_to_install,
                );
                match res {
                    Ok(()) => (),
                    Err(e) => {
                        tracing::error!("error while downloading the patch or applying it: {e}");
                        let _ = tx.send(Action::DownloadAndPatchError(e));
                    }
                }
            });
        }
    }

    fn progress_bars(&self, ui: &mut Ui) {
        match self.progress {
            Progress::Updating {
                ref current,
                versions,
            } => {
                ui.add(ProgressBar::new(versions).show_percentage());
                if let Some(current) = current.as_ref() {
                    ui.code(format!("Application du patch sur {}", current.display()));
                }
            }
            Progress::NotUpdating => (),
            Progress::Updated => {
                ui.label("Mise à jour complétée avec succès !");
            }
        }
    }

    fn display_error(&self, ui: &mut Ui) {
        if let Some(ref error) = self.download_error {
            ui.code(RichText::new(error.to_string()).color(Color32::RED));
        }
    }

    pub fn update(&mut self, ui: &mut Ui) {
        self.execute_instructions_from_receiver();

        ui.vertical_centered(|ui| {
            ui.heading("Gestionnaire de Mise à Jour");
            self.show_version(ui);
            ui.add_space(15.);
            self.file_selector(ui);
            self.apply_patch(ui);
            self.progress_bars(ui);
            self.display_error(ui);
        });
    }
}
