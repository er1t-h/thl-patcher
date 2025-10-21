#![windows_subsystem = "windows"]

use std::{
    fs::File,
    io::{self, BufWriter},
    sync::mpsc::{self, Receiver},
};

mod file_selectors;

use eframe::egui::{self, Color32, ProgressBar, RichText};
use xz2::write::XzEncoder;

use crate::file_selectors::{FileSelectors, FileTriplet};

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
    Error(io::Error),
}

struct MyApp {
    file_selector: FileSelectors,
    progress: Option<f32>,
    rx: Option<Receiver<Message>>,
    error: Option<io::Error>
}

impl MyApp {
    fn new() -> Self {
        Self {
            file_selector: FileSelectors::new(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Updated {
    Yes,
    No
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_messages();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let can_modify = match self.progress {
                    None => true,
                    Some(1.) => true,
                    Some(_) => false
                };
                ui.add_enabled_ui(can_modify, |ui| {
                    if self.file_selector.display(ui) == Updated::Yes {
                        self.progress = None
                    }
                });

                if let Some(FileTriplet { original, new, result }) = self.file_selector.triplet()
                    && ui.button("Créer le patch").clicked()
                {
                    let ctx = ui.ctx().clone();
                    let (tx, rx) = mpsc::channel();
                    self.rx = Some(rx);
                    let original = original.to_path_buf();
                    let new = new.to_path_buf();
                    let result = result.to_path_buf();
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
                    if progress == 1. {
                        ui.label("Patch créé avec succès !");
                    }
                }

                if let Some(ref error) = self.error {
                    ui.code(RichText::new(format!("Erreur en créant le patch : {error}")).color(Color32::RED));
                }
            });
        });
    }
}
