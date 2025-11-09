use std::{
    path::Path,
};

use indicatif::{MultiProgress, ProgressBar};
use patcher_common::download::ProgressReporter;

use patcher_common::{
    structures::{config::PatcherConfig, source::Source},
};

fn get_config() -> PatcherConfig {
    if let Ok(file) = std::fs::read_to_string("config.yaml")
        && let Ok(config) = serde_yaml::from_str(&file)
    {
        config
    } else {
        PatcherConfig::default()
    }
}

struct Progress {
    _multi: MultiProgress,
    bar: ProgressBar,
    sub: ProgressBar
}

impl Progress {
    fn new(len: u64) -> Self {
        let bar = ProgressBar::new(len);
        let sub = ProgressBar::new_spinner();
        let multi = MultiProgress::new();
        multi.add(bar.clone());
        multi.add(sub.clone());
        Self { _multi: multi, bar, sub }
    }
}

impl ProgressReporter for Progress {
    fn on_start_new_version(&mut self, transition: &patcher_common::structures::source::VersionTransitionRef) {
        self.sub.set_message(format!("downloading {}", transition.new.name));
    }

    fn on_patching_file(&mut self, path: &Path) {
        self.sub.set_message(format!("patching {}", path.display()));
    }

    fn on_version_patch_end(&mut self) {
        self.bar.inc(1);
    }

    fn on_finish(&mut self) {
        self.sub.finish_and_clear();
        self.bar.finish();
    }
}

fn main() {
    let _ = log4rs::init_file("log4rs.yaml", Default::default());
    let config = get_config();
    let source = match Source::from_url(&config.source) {
        Ok(s) => {
            log::debug!("source fetched successfully");
            s
        }
        Err(e) => {
            log::error!("error while fetching source: {e}");
            return;
        }
    };

    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(x) => x,
        Err(e) => {
            log::error!("couldn't initialize readline editor: {e}");
            return;
        }
    };

    println!("Écrivez le chemin vers le dossier de votre jeu.");
    let s = match config.get_default_path() {
        Some(x) => {
            println!("Un chemin par défaut a été trouvé. Confirmez en appuyant sur Entrée.");
            rl.readline_with_initial("> ", (x.as_str(), ""))
        }
        None => rl.readline("> "),
    };

    let path = match s {
        Ok(s) => s,
        Err(e) => {
            log::error!("error when trying to read line: {e}");
            return;
        }
    };
    let path = Path::new(&path);

    let current_version = match source.get_current_version(path) {
        Ok(Some(x)) => {
            log::debug!("found version {x}");
            x
        }
        Ok(None) => {
            log::error!("unknown or corrupted version");
            return;
        }
        Err(e) => {
            log::error!("error while fetching current version: {e}");
            return;
        }
    };

    println!("Version actuelle : {}", source.versions[current_version].name);
    let versions_to_install = source.get_transitions(current_version);
    println!("Dernière version : {}", source.versions.last().unwrap().name);
    if current_version + 1 == source.versions.len() {
        println!("Vous avez déjà la dernière version !");
        return;
    }
    println!("Souhaitez vous installer la dernière version ? [y/n]");

    let should_download = loop {
        match rl.readline("> ") {
            Ok(x) if ["y", "yes", "o", "oui"].contains(&x.as_str()) => break true,
            Ok(x) if ["n", "no", "non"].contains(&x.as_str()) => break false,
            Err(e) => {
                log::error!("readline error: {e}");
                return;
            }
            Ok(_) => ()
        }
    };

    if !should_download {
        log::info!("refused download");
        return;
    }

    let progress = Progress::new(versions_to_install.len() as u64);
    match patcher_common::download::download_and_patch(path, versions_to_install, progress) {
        Ok(()) => log::debug!("update successfully applied"),
        Err(e) => log::error!("update failed: {e}"),
    }
}
