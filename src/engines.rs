//! Persist locally added engine binaries (path only — no UCI connection yet).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "engines.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalEngine {
    pub id: String,
    pub name: String,
    pub path: String,
}

pub fn config_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("novelty");
    }
    PathBuf::from(".novelty")
}

pub fn engines_install_dir() -> PathBuf {
    config_dir().join("engines")
}

fn engines_path() -> PathBuf {
    config_dir().join(FILE_NAME)
}

pub fn load_engines() -> Vec<LocalEngine> {
    let path = engines_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut engines: Vec<LocalEngine> = serde_json::from_str(&raw).unwrap_or_default();
    let before = engines.len();
    engines.retain(|engine| Path::new(&engine.path).is_file());
    if engines.len() != before {
        let _ = save_engines(&engines);
    }
    engines
}

pub fn save_engines(engines: &[LocalEngine]) -> Result<(), String> {
    let path = engines_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let raw = serde_json::to_string_pretty(engines).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())
}

fn insert_engine(
    engines: &mut Vec<LocalEngine>,
    name: String,
    path: PathBuf,
) -> Result<(), String> {
    let path = path.canonicalize().unwrap_or(path);
    if !path.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }
    let path_str = path.to_string_lossy().into_owned();
    if engines.iter().any(|engine| engine.path == path_str) {
        return Err("already in list".into());
    }
    engines.push(LocalEngine {
        id: path_str.clone(),
        name,
        path: path_str,
    });
    Ok(())
}

pub fn add_engine(engines: &mut Vec<LocalEngine>, path: PathBuf) -> Result<(), String> {
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Engine")
        .to_string();
    insert_engine(engines, name, path)?;
    save_engines(engines)
}

pub fn register_engine(
    engines: &mut Vec<LocalEngine>,
    name: impl Into<String>,
    path: PathBuf,
) -> Result<(), String> {
    insert_engine(engines, name.into(), path)?;
    save_engines(engines)
}

pub fn remove_engine(engines: &mut Vec<LocalEngine>, id: &str) -> Result<(), String> {
    engines.retain(|engine| engine.id != id);
    save_engines(engines)
}

pub fn display_path(path: &str) -> String {
    let home = std::env::var("HOME").ok();
    if let Some(home) = home {
        let prefix = format!("{home}/");
        if let Some(rest) = path.strip_prefix(&prefix) {
            return format!("~/{rest}");
        }
    }
    path.to_string()
}

pub fn is_executable(path: &str) -> bool {
    let path = Path::new(path);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_path_shortens_home() {
        unsafe {
            std::env::set_var("HOME", "/Users/test");
        }
        assert_eq!(display_path("/Users/test/bin/stockfish"), "~/bin/stockfish");
    }

    #[test]
    fn insert_engine_deduplicates_by_path() {
        let dir = std::env::temp_dir().join(format!("novelty-engines-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("stockfish");
        fs::write(&file, b"bin").unwrap();

        let mut engines = Vec::new();
        insert_engine(&mut engines, "Stockfish".into(), file.clone()).unwrap();
        assert_eq!(engines.len(), 1);
        assert!(insert_engine(&mut engines, "Stockfish".into(), file).is_err());

        let _ = fs::remove_dir_all(dir);
    }
}
