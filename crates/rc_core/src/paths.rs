//! Assets root and user-data directory resolution (ARCHITECTURE §4.7).
//!
//! - assets: `--assets-dir` → env `RECHARGED_ASSETS` → `./assets` (if it exists) → `assets/` next
//!   to the executable;
//! - user data: `--user-data-dir` → env `RECHARGED_USER_DIR` → the OS config dir.
//!
//! Relative paths are resolved against the working directory, so every result is absolute.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// Environment variable overriding the assets root.
pub const ASSETS_ENV: &str = "RECHARGED_ASSETS";
/// Environment variable overriding the user-data directory.
pub const USER_DIR_ENV: &str = "RECHARGED_USER_DIR";

/// Resolved directories. Written only by the binary; read by game (saves, logs) and the data
/// loaders (track, vehicle, items, ai).
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct UserPaths {
    /// Saves, settings, ghosts, logs.
    pub data_dir: PathBuf,
    /// Root of all runtime content (`data/`, `models/`, `audio/`, …).
    pub assets_dir: PathBuf,
}

impl UserPaths {
    /// Resolves both directories from CLI values and the real process environment.
    pub fn resolve(cli_assets: Option<&Path>, cli_user: Option<&Path>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_owned));
        let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
        Self {
            assets_dir: resolve_assets_dir(cli_assets, &env, &cwd, exe_dir.as_deref(), &|p| {
                p.is_dir()
            }),
            data_dir: resolve_user_dir(cli_user, &env, &cwd),
        }
    }
}

/// Assets root by precedence (pure; `env` reads a variable, `is_dir` probes the file system).
pub fn resolve_assets_dir(
    cli: Option<&Path>,
    env: &dyn Fn(&str) -> Option<PathBuf>,
    cwd: &Path,
    exe_dir: Option<&Path>,
    is_dir: &dyn Fn(&Path) -> bool,
) -> PathBuf {
    if let Some(dir) = cli.map(Path::to_owned).or_else(|| env(ASSETS_ENV)) {
        return cwd.join(dir);
    }
    let local = cwd.join("assets");
    match exe_dir {
        Some(exe) if !is_dir(&local) => exe.join("assets"),
        _ => local,
    }
}

/// User-data directory by precedence (pure; `env` reads a variable).
pub fn resolve_user_dir(
    cli: Option<&Path>,
    env: &dyn Fn(&str) -> Option<PathBuf>,
    cwd: &Path,
) -> PathBuf {
    if let Some(dir) = cli.map(Path::to_owned).or_else(|| env(USER_DIR_ENV)) {
        return cwd.join(dir);
    }
    os_config_dir(env).unwrap_or_else(|| cwd.join("user-data"))
}

/// The per-user configuration directory of this OS, with the game's folder appended.
fn os_config_dir(env: &dyn Fn(&str) -> Option<PathBuf>) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        env("APPDATA").map(|d| d.join("ReCharged"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support/ReCharged"))
    } else {
        env("XDG_CONFIG_HOME")
            .filter(|d| d.is_absolute())
            .or_else(|| env("HOME").map(|h| h.join(".config")))
            .map(|d| d.join("recharged"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<PathBuf> {
        move |name| {
            vars.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| PathBuf::from(v))
        }
    }

    const CWD: &str = "/work";

    #[test]
    fn assets_dir_precedence() {
        let cwd = Path::new(CWD);
        let exe = Some(Path::new("/opt/game"));
        let exists = |_: &Path| true;
        let missing = |_: &Path| false;
        let env = env_of(&[(ASSETS_ENV, "/env/assets")]);
        let none = env_of(&[]);
        let cli = Some(Path::new("my-assets"));
        assert_eq!(
            resolve_assets_dir(cli, &env, cwd, exe, &exists),
            Path::new("/work/my-assets")
        );
        assert_eq!(
            resolve_assets_dir(None, &env, cwd, exe, &exists),
            Path::new("/env/assets")
        );
        assert_eq!(
            resolve_assets_dir(None, &none, cwd, exe, &exists),
            Path::new("/work/assets")
        );
        assert_eq!(
            resolve_assets_dir(None, &none, cwd, exe, &missing),
            Path::new("/opt/game/assets")
        );
        assert_eq!(
            resolve_assets_dir(None, &none, cwd, None, &missing),
            Path::new("/work/assets")
        );
    }

    #[test]
    fn user_dir_precedence() {
        let cwd = Path::new(CWD);
        let env = env_of(&[
            (USER_DIR_ENV, "/env/user"),
            ("HOME", "/home/p"),
            ("APPDATA", "C:/AppData"),
        ]);
        let cli = Some(Path::new("target/agent-data/T01"));
        assert_eq!(
            resolve_user_dir(cli, &env, cwd),
            Path::new("/work/target/agent-data/T01")
        );
        assert_eq!(resolve_user_dir(None, &env, cwd), Path::new("/env/user"));
        let os = resolve_user_dir(
            None,
            &env_of(&[("HOME", "/home/p"), ("APPDATA", "C:/AppData")]),
            cwd,
        );
        assert_ne!(os, Path::new("/env/user"));
        if cfg!(target_os = "linux") {
            assert_eq!(os, Path::new("/home/p/.config/recharged"));
            let xdg = env_of(&[("XDG_CONFIG_HOME", "/xdg"), ("HOME", "/home/p")]);
            assert_eq!(
                resolve_user_dir(None, &xdg, cwd),
                Path::new("/xdg/recharged")
            );
        }
        assert_eq!(
            resolve_user_dir(None, &env_of(&[]), cwd),
            Path::new("/work/user-data")
        );
    }
}
