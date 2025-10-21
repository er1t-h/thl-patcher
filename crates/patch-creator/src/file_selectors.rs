use std::path::{Path, PathBuf};

use eframe::egui::Ui;

use crate::Updated;

pub struct FileTriplet<'a> {
    pub original: &'a Path,
    pub new: &'a Path,
    pub result: &'a Path,
}

pub struct FileSelectors {
    original: Option<PathBuf>,
    new: Option<PathBuf>,
    result: Option<PathBuf>,
}

impl FileSelectors {
    pub const fn new() -> Self {
        Self {
            original: None,
            new: None,
            result: None,
        }
    }

    pub fn display(&mut self, ui: &mut Ui) -> Updated {
        ui.label("Dossier pré-mise-à-jour");
        let mut changed = Updated::No;
        if ui.button("Parcourir les fichiers...").clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            self.original = Some(path);
            changed = Updated::Yes;
        }
        if let Some(ref original) = self.original {
            ui.horizontal(|ui| {
                ui.label("Dossier séléctionné : ");
                ui.monospace(original.display().to_string());
            });
        }

        ui.label("Dossier contenant les fichiers mise-à-jour");
        if ui.button("Parcourir les fichiers...").clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            self.new = Some(path);
            changed = Updated::Yes;
        }
        if let Some(ref new) = self.new {
            ui.horizontal(|ui| {
                ui.label("Dossier séléctionné : ");
                ui.monospace(new.display().to_string());
            });
        }

        ui.label("Dossier de destination");
        if ui.button("Parcourir les fichiers...").clicked()
            && let Some(path) = rfd::FileDialog::new()
                .set_file_name("result.tar")
                .save_file()
        {
            self.result = Some(path);
            changed = Updated::Yes;
        }
        if let Some(ref result) = self.result {
            ui.horizontal(|ui| {
                ui.label("Fichier à créer : ");
                ui.monospace(result.display().to_string());
            });
        }
        changed
    }

    pub fn triplet(&self) -> Option<FileTriplet<'_>> {
        Some(FileTriplet {
            original: self.original.as_deref()?,
            new: self.new.as_deref()?,
            result: self.result.as_deref()?,
        })
    }
}
