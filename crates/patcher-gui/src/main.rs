use eframe::egui;

use crate::{
    structures::{config::PatcherConfig, source::Source},
    ui::{AppScreen, global_error::GlobalErrorType, patcher::Patcher},
};

mod error;
mod structures;
mod ui;

fn get_config() -> PatcherConfig {
    if let Ok(file) = std::fs::read_to_string("config.yaml")
        && let Ok(config) = serde_yaml::from_str(&file)
    {
        config
    } else {
        PatcherConfig::default()
    }
}

fn main() {
    let mut config = get_config();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        &std::mem::take(&mut config.window_name),
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(config)))),
    )
    .unwrap();
}

struct MyApp {
    global_state: (),
    app_screen: AppScreen,
}

impl MyApp {
    fn new(config: PatcherConfig) -> Self {
        let source: Result<Source, GlobalErrorType> = (|| {
            let body = minreq::get(&config.source).send()?;
            Ok(serde_yaml::from_slice(&body.as_bytes())?)
        })();

        match source {
            Ok(source) => {
                MyApp {
                    global_state: (),
                    app_screen: AppScreen::Patcher(Patcher::new(config, source)),
                }
            },
            Err(e) => MyApp {
                global_state: (),
                app_screen: AppScreen::source_error(e),
            },
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| match self.app_screen {
            AppScreen::SourceError(ref mut se) => se.update(ui),
            AppScreen::Patcher(ref mut patcher) => patcher.update(ui),
        });
    }

    // fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    //     egui::CentralPanel::default().show(ctx, |ui| {
    //         ui.vertical_centered(|ui| {
    //             ui.heading("Gestionnaire de Mise à Jour");

    //             if let Ok(x) = self.get_current_version() {
    //                 ui.label(format!(
    //                     "Version actuelle : {}",
    //                     &self.source.versions[x].name
    //                 ));
    //                 if self.source.versions.len() - 1 == x {
    //                     ui.label("Vous avez la dernière version !");
    //                 }
    //             } else {
    //                 ui.label(format!(
    //                     "Version actuelle non trouvée. Données corrompues ?"
    //                 ));
    //             }

    //             ui.add_space(20.);

    //             ui.label("Veuillez sélectionner le dossier du jeu");
    //             if ui.button("Parcourir les fichiers...").clicked()
    //                 && let Some(path) = rfd::FileDialog::new().pick_folder()
    //             {
    //                 *self.progress.lock().unwrap() = State::NotUpdating;
    //                 self.old = Some(path.display().to_string());
    //             }
    //             if let Some(old) = &self.old {
    //                 ui.label("Dossier séléctionné : ");
    //                 ui.monospace(old);
    //             }

    //             if let Some(ref old) = self.old {
    //                 if ui.button("Appliquer le Patch").clicked() {
    //                     let progress = Arc::clone(&self.progress);
    //                     let ui_version = Arc::clone(&self.version);
    //                     let current_version = self.get_current_version().unwrap();
    //                     let (_, new) = self
    //                         .source
    //                         .versions
    //                         .split_at_checked(current_version + 1)
    //                         .unwrap_or_default();
    //                     let new = new.to_vec();
    //                     let number_of_patch = new.len() as f32;
    //                     let old = old.to_string();
    //                     std::thread::spawn(move || {
    //                         for (i, version) in new.into_iter().enumerate() {
    //                             let versions = i as f32 / number_of_patch;
    //                             *progress.lock().unwrap() = State::Updating {
    //                                 versions,
    //                                 files: 0.,
    //                             };
    //                             // A temporary directory where patched files go
    //                             let temp_dir = tempdir().unwrap();
    //                             // A temporary directory to extract the archive
    //                             let patch = tempdir().unwrap();

    //                             let update_link = version.update_link.as_ref().unwrap();
    //                             let archive_content =
    //                                 minreq::get(update_link).send().unwrap().into_bytes();
    //                             let mut archive =
    //                                 ZipArchive::new(Cursor::new(archive_content)).unwrap();
    //                             archive.extract(patch.path()).unwrap();
    //                             let old = PathBuf::from(&old);

    //                             thl_patcher::patch(&old, &patch.path(), &temp_dir.path(), |s| {
    //                                 *progress.lock().unwrap() = State::Updating {
    //                                     versions,
    //                                     files: s.done as f32 / s.out_of as f32,
    //                                 };
    //                             });

    //                             for file in WalkDir::new(temp_dir.path()) {
    //                                 let file = file.unwrap();
    //                                 if !file.file_type().is_file() {
    //                                     continue;
    //                                 }
    //                                 let path = file.into_path();
    //                                 let suffix = path.strip_prefix(temp_dir.path()).unwrap();
    //                                 std::fs::copy(&path, old.join(suffix)).unwrap();
    //                             }
    //                             if let Some(x) = &mut *ui_version.lock().unwrap() {
    //                                 *x += 1;
    //                             }
    //                         }
    //                         *progress.lock().unwrap() = State::Updated;
    //                     });
    //                 }
    //             }

    //             match *self.progress.lock().unwrap() {
    //                 State::Updating { files, versions } => {
    //                     ui.add(ProgressBar::new(versions).show_percentage());
    //                     ui.add(ProgressBar::new(files).show_percentage());
    //                 }
    //                 State::NotUpdating => (),
    //                 State::Updated => {
    //                     ui.label("Mise à jour complétée avec succès");
    //                 }
    //             }
    //         });

    //         ui.ctx().request_repaint();
    //     });
    // }
}
