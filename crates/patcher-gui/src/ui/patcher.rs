use std::{
    io,
    path::Path,
    sync::mpsc::{self, Receiver},
};

use patcher_common::{download::ProgressReporter, error::DownloadAndPatchError, structures::{config::PatcherConfig, source::{Source, VersionTransition}}};
use eframe::egui::{Color32, ProgressBar, RichText, Ui};
use patcher_common::error::GetVersionError;

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
        done: u32,
        out_of: u32,
    },
    Updated,
}

pub struct Patcher {
    source: Source,
    progress: Progress,
    version: Version,
    selected_path: Option<String>,
    receiver: Option<Receiver<NewAction>>,
    sub_progressbar_text: Option<String>,
    download_error: Option<DownloadAndPatchError>,
}

enum NewAction {
    Downloading(String),
    Patching(String),
    FinishSingleVersion,
    Finish,
    DownloadAndPatchError(DownloadAndPatchError),
}

pub struct ProgressTracker {
    ctx: eframe::egui::Context,
    tx: std::sync::mpsc::Sender<NewAction>
}

impl ProgressReporter for ProgressTracker {
    fn on_start_new_version(&mut self, transition: &patcher_common::structures::source::VersionTransitionRef) {
        let _ = self.tx.send(NewAction::Downloading(transition.new.name.clone()));
        self.ctx.request_repaint();
    }

    fn on_patching_file(&mut self, path: &Path) {
        let _ = self.tx.send(NewAction::Patching(path.display().to_string()));
        self.ctx.request_repaint();
    }

    fn on_version_patch_end(&mut self) {
        let _ = self.tx.send(NewAction::FinishSingleVersion);
        self.ctx.request_repaint();
    }

    fn on_finish(&mut self) {
        let _ = self.tx.send(NewAction::Finish);
        self.ctx.request_repaint();
    }
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
            sub_progressbar_text: None,
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
                    NewAction::Downloading(name) => {
                        self.sub_progressbar_text = Some(format!("Téléchargement de la version {name}"));
                    }
                    NewAction::Patching(name) => {
                        self.sub_progressbar_text = Some(format!("Application du patch sur le fichier {name}"));
                    }
                    NewAction::FinishSingleVersion => {
                        if let Progress::Updating { done, .. } = &mut self.progress {
                            *done += 1;
                        }
                    }
                    NewAction::Finish => {
                        self.sub_progressbar_text = None;
                        self.progress = Progress::Updated;
                    }
                    NewAction::DownloadAndPatchError(error) => {
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

    #[allow(clippy::cast_possible_truncation)]
    fn apply_patch(&mut self, ui: &mut Ui) {
        if let Some(ref old) = self.selected_path
            && let Version::Found(current_version) = self.version
            && ui.button("Appliquer le Patch").clicked()
        {
            let versions_to_install: Vec<_> = self
                .source
                .get_transitions(current_version)
                .map(|x| x.to_owned())
                .collect();
            let old = old.clone();
            let (tx, rx) = mpsc::channel();
            let ctx = ui.ctx().clone();
            self.receiver = Some(rx);
            self.progress = Progress::Updating { done: 0, out_of: versions_to_install.len() as u32 };
            std::thread::spawn(move || {
                let res = patcher_common::download::download_and_patch(
                    Path::new(&old),
                    versions_to_install.iter().map(VersionTransition::as_ref),
                    ProgressTracker {
                        ctx,
                        tx: tx.clone()
                    }
                );
                match res {
                    Ok(()) => (),
                    Err(e) => {
                        log::error!("error while downloading the patch or applying it: {e}");
                        let _ = tx.send(NewAction::DownloadAndPatchError(e));
                    }
                }
            });
        }
    }

    fn progress_bars(&self, ui: &mut Ui) {
        match self.progress {
            Progress::Updating {
                done,
                out_of
            } => {
                #[allow(clippy::cast_precision_loss)]
                ui.add(ProgressBar::new(done as f32 / out_of as f32).show_percentage());
                if let Some(text) = self.sub_progressbar_text.as_ref() {
                    ui.code(text);
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
