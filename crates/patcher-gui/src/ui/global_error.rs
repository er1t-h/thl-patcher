use eframe::egui::{Color32, RichText, Ui};

pub enum GlobalErrorType {
    SourceNotFound(minreq::Error),
    SourceFormatError(serde_yaml::Error),
}
impl From<minreq::Error> for GlobalErrorType {
    fn from(value: minreq::Error) -> Self {
        Self::SourceNotFound(value)
    }
}
impl From<serde_yaml::Error> for GlobalErrorType {
    fn from(value: serde_yaml::Error) -> Self {
        Self::SourceFormatError(value)
    }
}

pub struct SourceError {
    pub error: GlobalErrorType,
}

impl SourceError {
    pub fn update(&mut self, ui: &mut Ui) {
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
