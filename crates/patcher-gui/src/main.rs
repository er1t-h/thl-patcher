#![windows_subsystem = "windows"]
#![warn(
    clippy::panic,
    clippy::unwrap_used,
    clippy::all,
    clippy::pedantic,
    clippy::nursery
)]

use std::process::ExitCode;
use eframe::egui;
use log4rs::config::Deserializers;
use crate::ui::{AppScreen, patcher::Patcher};
use patcher_common::structures::{config::PatcherConfig, source::Source};



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

fn main() -> ExitCode {
    let _ = log4rs::init_file("log4rs.yaml", Deserializers::default());

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
        Ok(()) => {
            log::debug!("Program exited successfuly");
            ExitCode::SUCCESS
        }
        Err(e) => {
            log::error!("couldn't initialize the window: {e}");
            ExitCode::FAILURE
        }
    }
}

struct MyApp {
    app_screen: AppScreen,
}

impl MyApp {
    fn new(config: &PatcherConfig) -> Self {
        match Source::from_url(&config.source) {
            Ok(source) => {
                log::debug!("source fetched successfully");
                Self {
                    app_screen: AppScreen::Patcher(Patcher::new(config, source)),
                }
            }
            Err(e) => {
                log::error!("error while fetching source: {e}");
                Self {
                    app_screen: AppScreen::source_error(e),
                }
            }
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
