//! # recharged — the game binary (BIN)
//!
//! Parses the command line ([`cli`]) into `AutomationConfig`, resolves `UserPaths` and starts the
//! windowed game assembled by the crate's library (`recharged::game`: plugins, React bundle,
//! automation hooks, `--bench`, the quit guard), or `--headless-race` ([`headless`]) and
//! `--calibrate-medals` ([`calibrate`]). `--export-bindings <path>` writes the TypeScript client
//! and exits without opening a window. Tools: `docs/design/automation.md`.
//!
//! Must never: contain game logic (it only wires plugins and flags together).

mod calibrate;
mod cli;
mod headless;

use bevy::prelude::*;
use rc_core::prelude::*;
use rc_input::script::InputScript;

fn main() -> AppExit {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match cli::parse(&args) {
        Ok(config) => config,
        Err(cli::CliError::Help) => {
            println!("{}", cli::USAGE);
            return AppExit::Success;
        }
        Err(cli::CliError::Invalid(message)) => {
            eprintln!("error: {message}\n\n{}", cli::USAGE);
            return AppExit::from_code(2);
        }
    };
    if let Some(path) = &config.export_bindings {
        return match rc_ui::export_bindings(path) {
            Ok(()) => {
                println!("wrote React bindings to {}", path.display());
                AppExit::Success
            }
            Err(e) => {
                eprintln!("cannot write {}: {e}", path.display());
                AppExit::error()
            }
        };
    }
    let paths = UserPaths::resolve(
        config.assets_dir.as_deref(),
        config.user_data_dir.as_deref(),
    );
    if config.calibrate_medals {
        return calibrate::run(paths);
    }
    if config.headless_race {
        return headless::race(config, paths);
    }
    let script = match config.input_script.as_deref().map(InputScript::load) {
        Some(Err(e)) => {
            eprintln!("error: --input-script {e}");
            return AppExit::from_code(2);
        }
        loaded => loaded.and_then(Result::ok),
    };
    let bundle = match recharged::checked_ui_bundle() {
        Ok(bundle) => bundle,
        Err(e) => {
            eprintln!("{e}");
            return AppExit::error();
        }
    };
    let mut app = recharged::game(config, paths, bundle);
    if let Some(script) = script {
        app.insert_resource(script);
    }
    app.run()
}
