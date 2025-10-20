use std::{
    fs::File,
    io::{self, BufWriter},
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

use eframe::egui::{self, Color32, ProgressBar, RichText};
use xz2::write::XzEncoder;

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 300.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Patch Creator",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new()))),
    )
    .unwrap();
}

enum Message {
    DiffState(thl_patcher::DiffState),
    Error(io::Error)
}

struct MyApp {
    original: Option<PathBuf>,
    new: Option<PathBuf>,
    result: Option<PathBuf>,
    progress: Option<f32>,
    rx: Option<Receiver<Message>>,
    error: Option<io::Error>
}

impl MyApp {
    fn new() -> Self {
        Self {
            original: None,
            new: None,
            result: None,
            progress: None,
            rx: None,
            error: None
        }
    }
}

impl MyApp {
    fn receive_messages(&mut self) {
        let mut should_stop = false;
        if let Some(ref rx) = self.rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::DiffState(x) => {
                        self.progress = Some(x.done as f32 / x.out_of as f32);
                        if x.done == x.out_of {
                            should_stop = true;
                        }
                    }
                    Message::Error(e) => {
                        self.error = Some(e);
                        should_stop = true;
                    }
                }
            }
        }
        if should_stop {
            self.rx = None;
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_messages();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label("Dossier pré-mise-à-jour");
                if ui.button("Parcourir les fichiers...").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.original = Some(path);
                    self.progress = None;
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
                    self.progress = None;
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
                    self.progress = None;
                }
                if let Some(ref result) = self.result {
                    ui.horizontal(|ui| {
                        ui.label("Fichier à créer : ");
                        ui.monospace(result.display().to_string());
                    });
                }

                if let Some(((original, new), result)) = self
                    .original
                    .as_ref()
                    .zip(self.new.as_ref())
                    .zip(self.result.as_ref())
                    && ui.button("Créer le patch").clicked()
                {
                    let ctx = ui.ctx().clone();
                    let (tx, rx) = mpsc::channel();
                    self.rx = Some(rx);
                    let original = original.clone();
                    let new = new.clone();
                    let result = result.clone();
                    std::thread::spawn(move || {
                        let error: Result<(), io::Error> = (|| {
                            let tar_archive = File::create(&result)?;
                            let encoder = XzEncoder::new(tar_archive, 9);
                            let mut tar = tar::Builder::new(BufWriter::new(encoder));
    
                            let _ = thl_patcher::diff_in_tar(&original, &new, &mut tar, |x| {
                                let _ = tx.send(Message::DiffState(x));
                                ctx.request_repaint();
                            });
    
                            tar.finish()?;
                            Ok(())
                        })();
                        if let Err(e) = error {
                            let _ = tx.send(Message::Error(e));
                            ctx.request_repaint();
                        }
                    });
                }

                if let Some(progress) = self.progress {
                    ui.add(ProgressBar::new(progress).show_percentage());
                }

                if let Some(ref error) = self.error {
                    ui.code(RichText::new(format!("Erreur en creant le patch : {error}")).color(Color32::RED));
                }
            });
        });
    }
}
