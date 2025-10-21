#![windows_subsystem = "windows"]
#![warn(clippy::panic, clippy::unwrap_used, clippy::all, clippy::pedantic, clippy::nursery)]

use std::{fs::File, io::Write, process::ExitCode};

use eframe::egui;
use tracing::level_filters::LevelFilter;

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

fn open_log() -> Box<dyn Write> {
    let file = File::options()
        .append(true)
        .create(true)
        .open("logs.txt");
    match file {
        Ok(x) => Box::new(x),
        Err(_) => Box::new(std::io::sink()),
    }
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(open_log)
        .with_max_level(LevelFilter::WARN)
        .init();

    let mut config = get_config();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 300.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    let res = eframe::run_native(
        &std::mem::take(&mut config.window_name),
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(&config)))),
    );
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("couldn't initialize the window: {e}");
            ExitCode::FAILURE
        }
    }
}

struct MyApp {
    app_screen: AppScreen,
}

impl MyApp {
    fn new(config: &PatcherConfig) -> Self {
        let source: Result<Source, GlobalErrorType> = (|| {
            let body = minreq::get(&config.source).send()?;
            Ok(serde_yaml::from_slice(body.as_bytes())?)
        })();

        match source {
            Ok(source) => Self {
                app_screen: AppScreen::Patcher(Patcher::new(config, source)),
            },
            Err(e) => {
                tracing::error!("error while fetching source: {}", e);
                Self {
                    app_screen: AppScreen::source_error(e),
                }
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
}
