//! Persist recently used usernames for quick re-selection.

use std::fs;
use std::path::PathBuf;

const MAX_PROFILES: usize = 24;
const FILE_NAME: &str = "profiles.json";

fn profiles_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("novelty")
            .join(FILE_NAME);
    }
    PathBuf::from(".novelty_profiles.json")
}

pub fn load_profiles() -> Vec<String> {
    let path = profiles_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save_profiles(profiles: &[String]) {
    let path = profiles_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(profiles) {
        let _ = fs::write(path, raw);
    }
}

pub fn remember_profile(profiles: &mut Vec<String>, username: &str) {
    let name = username.trim();
    if name.is_empty() {
        return;
    }
    profiles.retain(|entry| !entry.eq_ignore_ascii_case(name));
    profiles.insert(0, name.to_string());
    profiles.truncate(MAX_PROFILES);
    save_profiles(profiles);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remember_profile_deduplicates_and_moves_to_front() {
        let mut profiles = vec!["b".into(), "a".into()];
        remember_profile(&mut profiles, "c");
        assert_eq!(profiles, vec!["c", "b", "a"]);
        remember_profile(&mut profiles, "b");
        assert_eq!(profiles, vec!["b", "c", "a"]);
    }
}
