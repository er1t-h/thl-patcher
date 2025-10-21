use eframe::egui::{Color32, RichText, Ui};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GlobalErrorType {
    #[error("source not found: {0}")]
    SourceNotFound(#[from] minreq::Error),
    #[error("source format error: {0}")]
    SourceFormatError(#[from] serde_yaml::Error),
}

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
