use std::path::Path;
use std::process::ExitCode;

use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar};
use patcher_common::download::ProgressReporter;

use patcher_common::structures::{config::PatcherConfig, source::Source};
use rustyline::DefaultEditor;

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
    sub: ProgressBar,
}

impl Progress {
    fn new(len: u64) -> Self {
        let bar = ProgressBar::new(len);
        let sub = ProgressBar::new_spinner();
        let multi = MultiProgress::new();
        multi.add(bar.clone());
        multi.add(sub.clone());
        Self {
            _multi: multi,
            bar,
            sub,
        }
    }
}

impl ProgressReporter for Progress {
    fn on_start_new_version(
        &mut self,
        transition: &patcher_common::structures::source::VersionTransitionRef,
    ) {
        self.sub
            .set_message(format!("downloading {}", transition.new.name));
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

pub enum PatcherError {
    CannotFetchSource,
    Readline(rustyline::error::ReadlineError),
    UnkownOrCorruptedVersion,
    ErrorWhileFetchingVersion(std::io::Error),
    UpdateFailed,
}

fn inner(rl: &mut DefaultEditor) -> ExitCode {
    let config = get_config();
    let source = match Source::from_url(&config.source) {
        Ok(s) => {
            log::debug!("source fetched successfully");
            s
        }
        Err(e) => {
            log::error!("error while fetching source: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!("Écrivez le chemin vers le dossier de votre jeu.");
    let s = match config.get_default_path() {
        Some(x) => {
            println!(
                "Un chemin par défaut a été trouvé. Confirmez en appuyant sur {}.",
                "Entrée".bold()
            );
            rl.readline_with_initial("> ", (x.as_str(), ""))
        }
        None => rl.readline("> "),
    };

    let path = match s {
        Ok(s) => s,
        Err(e) => {
            log::error!("error when trying to read line: {e}");
            return ExitCode::FAILURE;
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
            return ExitCode::FAILURE;
        }
        Err(e) => {
            log::error!("error while fetching current version: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "Version actuelle : {}",
        source.versions[current_version].name
    );
    let versions_to_install = source.get_transitions(current_version);
    println!(
        "Dernière version : {}",
        source.versions.last().unwrap().name
    );
    if current_version + 1 == source.versions.len() {
        println!("Vous avez déjà la dernière version !");
        return ExitCode::SUCCESS;
    }
    println!("Souhaitez vous installer la dernière version ? [oui/non]");

    let should_download = loop {
        match rl.readline("> ") {
            Ok(x) if ["y", "yes", "o", "oui"].contains(&x.as_str()) => break true,
            Ok(x) if ["n", "no", "non"].contains(&x.as_str()) => break false,
            Err(e) => {
                log::error!("readline error: {e}");
                return ExitCode::FAILURE;
            }
            Ok(_) => (),
        }
    };

    if !should_download {
        log::info!("refused download");
        return ExitCode::FAILURE;
    }

    let progress = Progress::new(versions_to_install.len() as u64);
    match patcher_common::download::download_and_patch(path, versions_to_install, progress) {
        Ok(()) => {
            log::debug!("update successfully applied");
            ExitCode::SUCCESS
        }
        Err(e) => {
            log::error!("update failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let _ = log4rs::init_file("log4rs.yaml", Default::default());

    let mut rl = match rustyline::DefaultEditor::new() {
        Ok(x) => x,
        Err(e) => {
            log::error!("couldn't initialize readline editor: {e}");
            return ExitCode::FAILURE;
        }
    };

    let code = inner(&mut rl);

    if code == ExitCode::FAILURE {
        println!(
            "[{}] Aïe... On dirait que l'installation s'est mal passée. La meilleure chose à faire, c'est de regarder ce qui se trouve dans le fichier `logs.txt`, et, au besoin, de contacter le développeur... Désolé du désagrément :/",
            "Erreur".red()
        );
    }

    println!(
        "Merci d'avoir utilisé ce patcheur ! Appuyez sur la touche {} pour fermer cette fenêtre !",
        "Entrée".bold()
    );
    let _ = rl.readline("");
    code
}
