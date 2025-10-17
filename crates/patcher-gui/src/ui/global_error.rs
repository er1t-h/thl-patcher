use eframe::egui::{Color32, Response, RichText, Ui};

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
        match &self.error {
            GlobalErrorType::SourceNotFound(e) => {
                ui.colored_label(Color32::RED, "La source specifiee n'a pas ete trouvee");
                ui.code(RichText::new(e.to_string()).color(Color32::RED));
            }
            GlobalErrorType::SourceFormatError(e) => {
                ui.colored_label(Color32::RED, "Le format de la source est invalide");
                ui.code(RichText::new(e.to_string()).color(Color32::RED));
            }
        }
    }
}
