//! Lichess OAuth, session storage, and shared client helpers via [litchee].

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;

use litchee::LichessClient;
use litchee::api::auth::oauth::{AuthorizationRequest, CodeExchange, Scope};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};

use crate::engines::config_dir;

pub const CLIENT_ID: &str = "novelty";
const SESSION_FILE: &str = "lichess.json";

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LichessSession {
    pub username: String,
    pub access_token: String,
}

fn session_path() -> PathBuf {
    config_dir().join(SESSION_FILE)
}

pub fn load_session() -> Option<LichessSession> {
    let raw = fs::read_to_string(session_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_session(session: &LichessSession) -> Result<(), String> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let raw = serde_json::to_string_pretty(session).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())?;
    Ok(())
}

pub fn clear_session() -> Result<(), String> {
    let path = session_path();
    if path.is_file() {
        fs::remove_file(path).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub fn lichess_client(token: Option<&str>) -> Result<LichessClient, String> {
    let mut builder = LichessClient::builder();
    if let Some(token) = token {
        builder = builder.token(token);
    }
    builder.build().map_err(|err| err.to_string())
}

pub fn authenticate_in_browser(username: &str) -> Result<LichessSession, String> {
    block_on(authenticate_in_browser_async(username))
}

async fn authenticate_in_browser_async(username: &str) -> Result<LichessSession, String> {
    let username = username.trim();
    if username.is_empty() {
        return Err("Username required for Lichess login".into());
    }

    let client = lichess_client(None)?;
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|err| format!("Failed to start OAuth callback server: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("Failed to read OAuth callback address: {err}"))?;
    let redirect_uri = format!("http://{addr}/callback");

    let scopes = [Scope::PreferenceRead];
    let auth = client
        .oauth()
        .authorization_url(&AuthorizationRequest {
            client_id: CLIENT_ID,
            redirect_uri: &redirect_uri,
            scopes: &scopes,
            username_hint: Some(username),
        })
        .map_err(|err| err.to_string())?;

    open_browser(auth.url.as_str())?;

    let code = wait_for_redirect(listener, &auth.state).await?;
    let token = client
        .oauth()
        .exchange_code(&CodeExchange {
            code: &code,
            code_verifier: &auth.verifier,
            redirect_uri: &redirect_uri,
            client_id: CLIENT_ID,
        })
        .await
        .map_err(|err| err.to_string())?;
    let access_token = token.access_token.into_inner();

    let authed = lichess_client(Some(&access_token))?;
    let profile = authed
        .account()
        .profile()
        .await
        .map_err(|err| err.to_string())?;
    if !profile.user.username.eq_ignore_ascii_case(username) {
        return Err(format!(
            "Logged in as {} but expected {username}",
            profile.user.username
        ));
    }
    let session = LichessSession {
        username: profile.user.username,
        access_token,
    };
    save_session(&session)?;
    Ok(session)
}

/// Token for loading games: only when the requested user matches the logged-in account.
pub fn token_for_username<'a>(session: &'a LichessSession, username: &str) -> Option<&'a str> {
    if session.username.eq_ignore_ascii_case(username.trim()) {
        Some(&session.access_token)
    } else {
        None
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(all(unix, not(target_os = "macos")))]
    let program = "xdg-open";

    std::process::Command::new(program)
        .arg(url)
        .spawn()
        .map_err(|err| format!("Failed to open browser: {err}"))?;
    Ok(())
}

enum Callback {
    Code(String),
    Denied(String),
    Ignore,
}

async fn wait_for_redirect(listener: TcpListener, expected_state: &str) -> Result<String, String> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|err| format!("OAuth callback failed: {err}"))?;
        let target = read_request_target(&mut stream).await;
        match classify_request(&target, expected_state) {
            Callback::Code(code) => {
                write_http_response(
                    &mut stream,
                    "Authorized — you can close this tab.",
                )
                .await?;
                return Ok(code);
            }
            Callback::Denied(error) => {
                write_http_response(&mut stream, "Authorization failed — check Novelty.")
                    .await?;
                return Err(error);
            }
            Callback::Ignore => {
                let _ = write_http_response(&mut stream, "Waiting for authorization…").await;
            }
        }
    }
}

async fn read_request_target(stream: &mut TcpStream) -> String {
    let mut buf = [0u8; 2048];
    let Ok(Ok(n)) = timeout(Duration::from_secs(5), stream.read(&mut buf)).await else {
        return String::new();
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned()
}

fn classify_request(target: &str, expected_state: &str) -> Callback {
    let query = target.split('?').nth(1).unwrap_or("");
    let mut code = None;
    let mut state = None;
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = urlencoding::decode(value)
            .map(|cow| cow.into_owned())
            .unwrap_or_else(|_| value.to_string());
        match key {
            "code" => code = Some(value),
            "state" => state = Some(value),
            "error" => return Callback::Denied(format!("authorization denied: {value}")),
            _ => {}
        }
    }
    match code {
        Some(code) if state.as_deref() == Some(expected_state) => Callback::Code(code),
        _ => Callback::Ignore,
    }
}

async fn write_http_response(stream: &mut TcpStream, body: &str) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|err: io::Error| err.to_string())?;
    stream.flush().await.map_err(|err: io::Error| err.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_for_username_is_case_insensitive() {
        let session = LichessSession {
            username: "DrNykterstein".into(),
            access_token: "lip_test".into(),
        };
        assert!(token_for_username(&session, "drnykterstein").is_some());
        assert!(token_for_username(&session, "other").is_none());
    }
}
