use std::{
    env::current_dir, path::Path
};

use eframe::egui;
use serde::Deserialize;
use tempfile::tempdir;
use walkdir::WalkDir;

#[allow(dead_code)]
#[derive(Debug)]
enum Error {
    Walkdir(walkdir::Error),
    Io(std::io::Error),
    Rustyline(rustyline::error::ReadlineError),
}
impl From<walkdir::Error> for Error {
    fn from(value: walkdir::Error) -> Self {
        Self::Walkdir(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<rustyline::error::ReadlineError> for Error {
    fn from(value: rustyline::error::ReadlineError) -> Self {
        Self::Rustyline(value)
    }
}

#[derive(Debug, Deserialize, Clone)]
struct DefaultPaths {
    target_os: String,
    possible_paths: Vec<String>,
}
#[derive(Debug, Deserialize, Clone)]
struct PatcherConfig {
    window_name: String,
    default_paths: Vec<DefaultPaths>,
}
impl Default for PatcherConfig {
    fn default() -> Self {
        Self {
            window_name: String::from("Patcher"),
            default_paths: vec![],
        }
    }
}

impl PatcherConfig {
    fn get_default_path(&self) -> Option<String> {
        for entry in self.default_paths.iter().filter(|x| x.target_os == std::env::consts::OS) {
            for path in &entry.possible_paths {
                let path = shellexpand::tilde(path);
                let hypothesis = Path::new(path.as_ref());
                if hypothesis.exists() {
                    return Some(path.to_string())
                }
            }
        }
        None
    }
}

fn cli_mode(config: &PatcherConfig) -> Result<(), Error> {
    println!("Argh... Il semblerait que je ne puisse pas ouvrir de fenêtre... Pas de souci !");
    println!("On va faire ça à l'ancienne, dans le terminal :D");
    println!("Je vais avoir besoin du chemin du jeu...");
    let default = if let Some(default) = config.get_default_path() {
        println!("Oh, parfait, il y en a déjà un pré-rempli ! En théorie, t'as juste à valider !");
        default
    } else {
        String::new()
    };
    let mut editor = rustyline::DefaultEditor::new()?;
    let path = editor.readline_with_initial("Chemin d'installation du jeu : ", (&default, ""))?;
    let current = current_dir()?;
    let temp_dir = tempdir()?;
    let old = Path::new(&path);
    thl_patcher::patch(old, &current.join("patch"), &temp_dir.path());

    for file in WalkDir::new(temp_dir.path()) {
        let file = file?;
        if !file.file_type().is_file() {
            continue;
        }
        let path = file.into_path();
        // Unwrap never panics because `temp_dir` is always a parent of `file`
        let suffix = path.strip_prefix(temp_dir.path()).unwrap();
        std::fs::copy(&path, old.join(suffix))?;
    }
    Ok(())
}

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
    println!("Patcher fait par Er1t -> github");
    let config = get_config();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    if eframe::run_native(
        &config.window_name,
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(&config)))),
    )
    .is_err()
    {
        cli_mode(&config).unwrap();
    };
}

struct MyApp {
    old: Option<String>,
    applied: bool,
}

impl MyApp {
    fn new(config: &PatcherConfig) -> Self {
        Self {
            old: config.get_default_path(),
            applied: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Sélectionnez le dossier du jeu");
                if ui.button("Parcourir les fichiers...").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.applied = false;
                    self.old = Some(path.display().to_string());
                }
                if let Some(old) = &self.old {
                    ui.label("Dossier séléctionné : ");
                    ui.monospace(old);
                }

                if let Some(ref old) = self.old {
                    if ui.button("Appliquer le Patch").clicked() {
                        ui.spinner();
                        let current = current_dir().unwrap();
                        let temp_dir = tempdir().unwrap();
                        let old = Path::new(old);
                        thl_patcher::patch(old, &current.join("patch"), &temp_dir.path());

                        for file in WalkDir::new(temp_dir.path()) {
                            let file = file.unwrap();
                            if !file.file_type().is_file() {
                                continue;
                            }
                            let path = file.into_path();
                            let suffix = path.strip_prefix(temp_dir.path()).unwrap();
                            std::fs::copy(&path, old.join(suffix)).unwrap();
                        }

                        self.old = None;
                        self.applied = true;
                    }
                }

                if self.applied {
                    ui.label("Patch appliqué avec succès !");
                }
            })
        });
    }
}
