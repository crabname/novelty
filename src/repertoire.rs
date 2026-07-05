//! Repertoire PGN files on disk (`~/.config/novelty/repertoires/`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::engines::config_dir;
use crate::graph::start_fen;
use crate::move_tree::MoveTree;
use crate::pgn_tree::format_repertoire_pgn;

pub fn repertoires_dir() -> PathBuf {
    config_dir().join("repertoires")
}

pub fn sanitize_filename(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn initial_headers(name: &str) -> Vec<(String, String)> {
    vec![
        ("Event".into(), name.to_string()),
        ("Site".into(), "?".into()),
        ("Date".into(), "????.??.??".into()),
        ("White".into(), "?".into()),
        ("Black".into(), "?".into()),
        ("Result".into(), "*".into()),
        ("Orientation".into(), "white".into()),
    ]
}

pub fn create_repertoire(name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Repertoire name required".into());
    }
    let slug = sanitize_filename(name);
    if slug.is_empty() {
        return Err("Invalid repertoire name".into());
    }
    let dir = repertoires_dir();
    fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
    let path = dir.join(format!("{slug}.pgn"));
    if path.is_file() {
        return Err(format!("Repertoire “{name}” already exists"));
    }
    let headers = initial_headers(name);
    let tree = MoveTree::from_fen(start_fen());
    let pgn = format_repertoire_pgn(&headers, &tree);
    fs::write(&path, pgn).map_err(|err| err.to_string())?;
    Ok(path)
}

pub fn save_repertoire(path: &Path, pgn: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(path, pgn).map_err(|err| err.to_string())
}

pub fn load_repertoire_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("Failed to read repertoire: {err}"))
}

pub fn list_repertoires() -> Vec<PathBuf> {
    let dir = repertoires_dir();
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "pgn"))
        .collect();
    paths.sort_by(|a, b| {
        let a_name = a.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let b_name = b.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        a_name.cmp(b_name)
    });
    paths
}

pub fn repertoire_display_name(path: &Path) -> String {
    if let Ok(text) = load_repertoire_file(path) {
        if let Ok(game) = crate::pgn_tree::parse_repertoire_pgn(&text) {
            if let Some(event) = game.headers.get("Event") {
                return event.clone();
            }
            return game.label;
        }
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Repertoire")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_alphanumeric() {
        assert_eq!(sanitize_filename("Caro-Kann"), "Caro-Kann");
        assert_eq!(sanitize_filename("  caro  "), "caro");
    }
}
