use eframe::egui::{Color32, RichText, Ui};
use patcher_common::error::GlobalErrorType;

pub struct SourceError {
    pub error: GlobalErrorType,
}

impl SourceError {
    pub fn update(&self, ui: &mut Ui) {
        let e = match &self.error {
            GlobalErrorType::SourceNotFound(e) => {
                ui.colored_label(Color32::RED, "La source spécifiée n'a pas été trouvée.");
                e.to_string()
            }
            GlobalErrorType::SourceFormatError(e) => {
                ui.colored_label(Color32::RED, "Le format de la source est invalide.");
                e.to_string()
            }
        };
        ui.code(RichText::new(e).color(Color32::RED));
    }
}
