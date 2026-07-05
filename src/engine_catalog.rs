//! Downloadable engines from GitHub Releases.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::engines::{engines_install_dir, LocalEngine};

#[derive(Clone, Debug)]
pub struct CatalogEngine {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub repo: &'static str,
}

#[derive(Clone, Debug)]
pub struct CatalogOffer {
    pub engine: CatalogEngine,
    pub version: String,
    pub asset_name: String,
    pub download_url: String,
    pub size: Option<u64>,
    pub available: bool,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

const USER_AGENT: &str = "novelty-chess-app";

pub fn catalog_engines() -> &'static [CatalogEngine] {
    static ENGINES: [CatalogEngine; 3] = [
        CatalogEngine {
            id: "stockfish",
            name: "Stockfish",
            description: "Strong open-source chess engine",
            repo: "official-stockfish/Stockfish",
        },
        CatalogEngine {
            id: "reckless",
            name: "Reckless",
            description: "Fast UCI engine by Code Delivery Service",
            repo: "codedeliveryservice/Reckless",
        },
        CatalogEngine {
            id: "lc0",
            name: "Leela Chess Zero",
            description: "Neural network chess engine",
            repo: "LeelaChessZero/lc0",
        },
    ];
    &ENGINES
}

pub fn resolve_catalog() -> Result<Vec<CatalogOffer>, String> {
    catalog_engines()
        .iter()
        .map(resolve_offer)
        .collect()
}

fn resolve_offer(engine: &CatalogEngine) -> Result<CatalogOffer, String> {
    let (version, assets) = fetch_release_assets(engine.repo)?;
    let preferences = asset_preferences(engine.id);
    let asset = pick_asset(&assets, &preferences);
    Ok(CatalogOffer {
        engine: engine.clone(),
        version,
        asset_name: asset.map(|a| a.name.clone()).unwrap_or_default(),
        download_url: asset
            .map(|a| a.browser_download_url.clone())
            .unwrap_or_default(),
        size: asset.map(|a| a.size),
        available: asset.is_some(),
    })
}

pub fn install_catalog_engine(offer: &CatalogOffer) -> Result<(String, PathBuf), String> {
    if !offer.available {
        return Err(format!(
            "{} is not available for this platform",
            offer.engine.name
        ));
    }

    let bytes = download_bytes(&offer.download_url)?;
    let install_dir = engines_install_dir()
        .join(offer.engine.id)
        .join(sanitize_version(&offer.version));
    fs::create_dir_all(&install_dir).map_err(|err| err.to_string())?;

    let binary_path = materialize_asset(&install_dir, &offer.asset_name, &bytes)?;
    set_executable(&binary_path)?;

    let display_name = format!("{} {}", offer.engine.name, offer.version);
    Ok((display_name, binary_path))
}

pub fn is_catalog_installed(engines: &[LocalEngine], catalog_id: &str) -> bool {
    let prefix = engines_install_dir().join(catalog_id);
    let prefix = prefix.to_string_lossy().to_string();
    engines
        .iter()
        .any(|engine| engine.path.starts_with(&prefix) && Path::new(&engine.path).is_file())
}

fn fetch_release_assets(repo: &str) -> Result<(String, Vec<GithubAsset>), String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| err.to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }
    let release: GithubRelease = response.json().map_err(|err| err.to_string())?;
    Ok((release.tag_name, release.assets))
}

fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|err| err.to_string())?;
    let response = client
        .get(url)
        .send()
        .map_err(|err| err.to_string())?;
    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }
    response.bytes().map(|b| b.to_vec()).map_err(|err| err.to_string())
}

fn materialize_asset(
    install_dir: &Path,
    asset_name: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    if asset_name.ends_with(".zip") {
        extract_zip(install_dir, bytes)?;
    } else if asset_name.ends_with(".tar") {
        extract_tar(install_dir, bytes)?;
    } else {
        let path = install_dir.join(asset_name);
        fs::write(&path, bytes).map_err(|err| err.to_string())?;
        return Ok(path);
    }
    find_binary(install_dir, asset_name)
}

fn extract_zip(dir: &Path, bytes: &[u8]) -> Result<(), String> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|err| err.to_string())?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|err| err.to_string())?;
        let outpath = dir.join(file.mangled_name());
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|err| err.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            let mut outfile = fs::File::create(&outpath).map_err(|err| err.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn extract_tar(dir: &Path, bytes: &[u8]) -> Result<(), String> {
    let cursor = Cursor::new(bytes);
    let mut archive = tar::Archive::new(cursor);
    archive.unpack(dir).map_err(|err| err.to_string())
}

fn find_binary(install_dir: &Path, asset_name: &str) -> Result<PathBuf, String> {
    let stem = asset_name
        .trim_end_matches(".tar")
        .trim_end_matches(".zip");
    let direct = install_dir.join(stem);
    if direct.is_file() {
        return Ok(direct);
    }

    let mut candidates = Vec::new();
    collect_files(install_dir, &mut candidates, 0)?;
    if candidates.len() == 1 {
        return Ok(candidates.pop().expect("one file"));
    }

    for path in &candidates {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == stem || name.contains(stem) {
            return Ok(path.clone());
        }
    }

    candidates
        .into_iter()
        .find(|path| is_executable_file(path))
        .ok_or_else(|| format!("Could not find engine binary in {}", install_dir.display()))
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) -> Result<(), String> {
    if depth > 4 {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_file() {
            out.push(path);
        } else if path.is_dir() {
            collect_files(&path, out, depth + 1)?;
        }
    }
    Ok(())
}

fn is_executable_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.starts_with('.') || name.ends_with(".dll") {
        return false;
    }
    #[cfg(windows)]
    {
        return name.ends_with(".exe") || !name.contains('.');
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        is_executable(path)
    }
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).map_err(|err| err.to_string())?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn pick_asset<'a>(assets: &'a [GithubAsset], preferences: &[&str]) -> Option<&'a GithubAsset> {
    for preference in preferences {
        if let Some(asset) = assets.iter().find(|asset| asset.name == *preference) {
            return Some(asset);
        }
        if let Some(prefix) = preference.strip_suffix('*')
            && let Some(asset) = assets.iter().find(|asset| asset.name.starts_with(prefix))
        {
            return Some(asset);
        }
    }
    None
}

fn asset_preferences(engine_id: &str) -> Vec<&'static str> {
    match (engine_id, std::env::consts::OS, std::env::consts::ARCH) {
        ("stockfish", "macos", "aarch64") => {
            vec!["stockfish-macos-m1-apple-silicon.tar"]
        }
        ("stockfish", "macos", _) => vec![
            "stockfish-macos-x86-64-avx2.tar",
            "stockfish-macos-x86-64-sse41-popcnt.tar",
            "stockfish-macos-x86-64.tar",
        ],
        ("stockfish", "linux", _) => vec![
            "stockfish-ubuntu-x86-64-avx2.tar",
            "stockfish-ubuntu-x86-64-sse41-popcnt.tar",
            "stockfish-ubuntu-x86-64.tar",
        ],
        ("stockfish", "windows", _) => vec![
            "stockfish-windows-x86-64-avx2.zip",
            "stockfish-windows-x86-64-sse41-popcnt.zip",
        ],
        ("reckless", "macos", _) => vec!["reckless-macos"],
        ("reckless", "linux", _) => vec![
            "reckless-linux-avx2",
            "reckless-linux-generic",
        ],
        ("reckless", "windows", _) => vec![
            "reckless-windows-avx2.exe",
            "reckless-windows-generic.exe",
        ],
        ("lc0", "windows", _) => vec![
            "lc0-v*-windows-cpu-openblas.zip",
            "lc0-v*-windows-cpu-dnnl.zip",
        ],
        _ => Vec::new(),
    }
}

fn sanitize_version(version: &str) -> String {
    version
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
            ch
        } else {
            '_'
        })
        .collect()
}

fn is_executable(path: &Path) -> bool {
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

pub fn format_bytes(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{size} B")
    }
}
