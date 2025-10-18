use eframe::egui;

use crate::{
    structures::{config::PatcherConfig, source::Source},
    ui::{AppScreen, global_error::GlobalErrorType, patcher::Patcher},
};

mod error;
mod structures;
mod transmitter_reloader;
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
            .with_inner_size([640.0, 300.0])
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
    app_screen: AppScreen,
}

impl MyApp {
    fn new(config: PatcherConfig) -> Self {
        let source: Result<Source, GlobalErrorType> = (|| {
            let body = minreq::get(&config.source).send()?;
            Ok(serde_yaml::from_slice(&body.as_bytes())?)
        })();

        match source {
            Ok(source) => MyApp {
                app_screen: AppScreen::Patcher(Patcher::new(config, source)),
            },
            Err(e) => MyApp {
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
}
