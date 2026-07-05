//! Repertoire PGN files on disk (`~/.config/novelty/repertoires/`).

use std::fs;
use std::path::{Path, PathBuf};

use crate::engines::config_dir;
use crate::fetch::{PlayerColor, Site};
use crate::graph::start_fen;
use crate::move_tree::MoveTree;
use crate::pgn_tree::format_repertoire_pgn;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedProfile {
    pub username: String,
    pub site: Site,
}

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

pub fn initial_headers(name: &str, color: PlayerColor) -> Vec<(String, String)> {
    vec![
        ("Event".into(), name.to_string()),
        ("Site".into(), "?".into()),
        ("Date".into(), "????.??.??".into()),
        ("White".into(), "?".into()),
        ("Black".into(), "?".into()),
        ("Result".into(), "*".into()),
        (
            "Orientation".into(),
            color.orientation_value().to_string(),
        ),
    ]
}

pub fn player_color_from_headers(headers: &[(String, String)]) -> PlayerColor {
    headers
        .iter()
        .find(|(tag, _)| tag == "Orientation")
        .map(|(_, value)| PlayerColor::from_orientation(value))
        .unwrap_or(PlayerColor::White)
}

pub fn set_linked_profile(
    headers: &mut Vec<(String, String)>,
    username: &str,
    site: Site,
    color: PlayerColor,
) {
    let username = username.trim();
    if username.is_empty() {
        return;
    }
    let site_value = match site {
        Site::Lichess => format!("https://lichess.org/@/{username}"),
        Site::ChessCom => format!("https://www.chess.com/member/{username}"),
    };
    set_header(headers, "Site", &site_value);
    match color {
        PlayerColor::White => {
            set_header(headers, "White", username);
            set_header(headers, "Black", "?");
        }
        PlayerColor::Black => {
            set_header(headers, "White", "?");
            set_header(headers, "Black", username);
        }
    }
}

pub fn linked_profile_from_headers(headers: &[(String, String)]) -> Option<LinkedProfile> {
    let username = profile_username_from_headers(headers)?;
    let site = profile_site_from_headers(headers)?;
    Some(LinkedProfile { username, site })
}

pub fn profile_username_from_headers(headers: &[(String, String)]) -> Option<String> {
    let color = player_color_from_headers(headers);
    let tag = match color {
        PlayerColor::White => "White",
        PlayerColor::Black => "Black",
    };
    headers
        .iter()
        .find(|(name, _)| name == tag)
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "?")
}

pub fn profile_site_from_headers(headers: &[(String, String)]) -> Option<Site> {
    let site = headers
        .iter()
        .find(|(name, _)| name == "Site")
        .map(|(_, value)| value.to_ascii_lowercase())?;
    if site.contains("chess.com") {
        Some(Site::ChessCom)
    } else if site.contains("lichess.org") {
        Some(Site::Lichess)
    } else {
        None
    }
}

/// Updates `[ECO]` and `[Opening]` from the mainline position history.
pub fn sync_opening_headers(headers: &mut Vec<(String, String)>, tree: &MoveTree) {
    if let Some(opening) = tree.mainline_opening() {
        set_header(headers, "ECO", &opening.eco);
        set_header(headers, "Opening", &opening.name);
    } else {
        remove_header(headers, "ECO");
        remove_header(headers, "Opening");
    }
}

fn set_header(headers: &mut Vec<(String, String)>, tag: &str, value: &str) {
    if let Some((_, existing)) = headers.iter_mut().find(|(name, _)| name == tag) {
        *existing = value.to_string();
    } else {
        headers.push((tag.to_string(), value.to_string()));
    }
}

fn remove_header(headers: &mut Vec<(String, String)>, tag: &str) {
    headers.retain(|(name, _)| name != tag);
}

pub fn create_repertoire(name: &str, color: PlayerColor) -> Result<PathBuf, String> {
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
    let headers = initial_headers(name, color);
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
    if let Ok(text) = load_repertoire_file(path)
        && let Ok(game) = crate::pgn_tree::parse_repertoire_pgn(&text)
    {
        if let Some(event) = game.headers.get("Event") {
            return event.clone();
        }
        return game.label;
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Repertoire")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetch::PlayerColor;
    use crate::pgn_tree::parse_repertoire_pgn;

    #[test]
    fn sanitize_keeps_alphanumeric() {
        assert_eq!(sanitize_filename("Caro-Kann"), "Caro-Kann");
        assert_eq!(sanitize_filename("  caro  "), "caro");
    }

    #[test]
    fn mainline_opening_and_save_headers_for_e4_e5() {
        let pgn = r#"[Event "King's Pawn"]

1. e4 e5 *"#;
        let parsed = parse_repertoire_pgn(pgn).expect("parse repertoire");
        let tree = parsed.tree;
        let opening = tree.mainline_opening().expect("e4 e5 opening");
        assert_eq!(opening.eco, "C20");

        let mut headers = initial_headers("King's Pawn", PlayerColor::White);
        sync_opening_headers(&mut headers, &tree);

        assert_eq!(
            headers
                .iter()
                .find(|(tag, _)| tag == "ECO")
                .map(|(_, value)| value.as_str()),
            Some("C20")
        );
        assert_eq!(
            headers
                .iter()
                .find(|(tag, _)| tag == "Opening")
                .map(|(_, value)| value.as_str()),
            Some(opening.name.as_str())
        );

        let pgn = format_repertoire_pgn(&headers, &tree);
        assert!(pgn.contains("[ECO \"C20\"]"));
        assert!(pgn.contains(&format!("[Opening \"{}\"]", opening.name)));
    }
}
