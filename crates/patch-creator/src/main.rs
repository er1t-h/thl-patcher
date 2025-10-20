use std::{fs::File, io::BufWriter, path::PathBuf, sync::mpsc::{self, Receiver}};

use eframe::egui::{self, ProgressBar};
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

struct MyApp {
    original: Option<PathBuf>,
    new: Option<PathBuf>,
    result: Option<PathBuf>,
    progress: Option<f32>,
    rx: Option<Receiver<thl_patcher::DiffState>>
}

impl MyApp {
    fn new() -> Self {
        Self { 
            original: None,
            new: None,
            result: None,
            progress: None,
            rx: None
         }
    }
}

impl MyApp {
    fn receive_messages(&mut self) {
        let mut should_stop = false;
        if let Some(ref rx) = self.rx {
            while let Ok(x) = rx.try_recv() {
                self.progress = Some(x.done as f32 / x.out_of as f32);
                if x.done == x.out_of {
                    should_stop = true;
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
                    && let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.original = Some(path)
                }
                if let Some(ref original) = self.original {
                    ui.horizontal(|ui| {
                        ui.label("Dossier séléctionné : ");
                        ui.monospace(original.display().to_string());
                    });
                }

                ui.label("Dossier contenant les fichiers mise-à-jour");
                if ui.button("Parcourir les fichiers...").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.new = Some(path)
                }
                if let Some(ref new) = self.new {
                    ui.horizontal(|ui| {
                        ui.label("Dossier séléctionné : ");
                        ui.monospace(new.display().to_string());
                    });
                }
                
                ui.label("Dossier de destination");
                if ui.button("Parcourir les fichiers...").clicked()
                    && let Some(path) = rfd::FileDialog::new().set_file_name("result.tar").save_file() {
                    self.result = Some(path)
                }
                if let Some(ref result) = self.result {
                    ui.horizontal(|ui| {
                        ui.label("Fichier à créer : ");
                        ui.monospace(result.display().to_string());
                    });
                }

                if 
                    let Some(((original, new), result)) = self.original.as_ref().zip(self.new.as_ref()).zip(self.result.as_ref())
                    && ui.button("Créer le patch").clicked()
                {
                    let ctx = ui.ctx().clone();
                    let (tx, rx) = mpsc::channel();
                    self.rx = Some(rx);
                    let original = original.clone();
                    let new = new.clone();
                    let result = result.clone();
                    std::thread::spawn(move || {
                        let tar_archive = File::create(&result).unwrap();
                        let encoder = XzEncoder::new(tar_archive, 9);
                        let mut tar = tar::Builder::new(BufWriter::new(encoder));

                        let _ = thl_patcher::diff_in_tar(&original, &new, &mut tar, |x| {
                            let _ = tx.send(x);
                            ctx.request_repaint();
                        });

                        tar.finish().unwrap();
                    });
                }

                if let Some(progress) = self.progress {
                    ui.add(ProgressBar::new(progress).show_percentage());
                }
            });
        });
    }
}
