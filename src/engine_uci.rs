//! UCI engine process communication via the `ruci` crate (shakmaty-based).

use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use gpui_chessboard::EvalDisplay;
use ruci::engine::{self, Info};
use ruci::gui::SetOption;
use ruci::{Engine, Go};
use ruci::gui::Position;
use std::str::FromStr;

use crate::eval_display::eval_from_score;
use crate::graph::turn_color;

#[derive(Clone, Debug)]
pub struct EngineLine {
    pub rank: u32,
    pub pv: Vec<String>,
    pub score: Option<String>,
    pub eval: Option<EvalDisplay>,
}

#[derive(Clone, Debug)]
pub struct AnalysisResult {
    pub lines: Vec<EngineLine>,
    pub depth: u32,
}

impl AnalysisResult {
    pub fn best_eval(&self) -> Option<EvalDisplay> {
        self.lines.first().and_then(|line| line.eval)
    }

    pub fn summary(&self) -> String {
        let Some(line) = self.lines.first() else {
            return "No analysis".into();
        };
        let mut parts = vec![format!("#{} {}", line.rank, line.pv.first().cloned().unwrap_or_default())];
        if let Some(score) = &line.score {
            parts.push(score.clone());
        }
        parts.push(format!("depth {}", self.depth));
        parts.join(" · ")
    }
}

#[derive(Clone, Debug)]
pub struct AnalysisRequest {
    pub fen: String,
    pub depth: u32,
    pub line_count: u32,
}

#[derive(Clone, Debug)]
pub enum UciEvent {
    Analysis(AnalysisResult),
    Error(String),
    Disconnected,
}

enum WorkerCommand {
    Analyze(AnalysisRequest),
    Disconnect,
}

pub struct UciSession {
    command_tx: Sender<WorkerCommand>,
    thread: Option<JoinHandle<()>>,
}

impl UciSession {
    pub fn connect(path: PathBuf, event_tx: Sender<UciEvent>) -> Result<(Self, String), String> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(RunningEngine::spawn(&path));
        });

        let running = match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(engine)) => engine,
            Ok(Err(err)) => return Err(err),
            Err(_) => {
                return Err(
                    "Engine connection timed out — is the binary executable and UCI-compatible?"
                        .into(),
                );
            }
        };

        let name = running.name.clone();

        let (command_tx, command_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("novelty-uci".into())
            .spawn(move || worker_loop(running, command_rx, event_tx))
            .map_err(|err| err.to_string())?;

        Ok((
            Self {
                command_tx,
                thread: Some(thread),
            },
            name,
        ))
    }

    pub fn analyze(&self, request: AnalysisRequest) -> Result<(), String> {
        self.command_tx
            .send(WorkerCommand::Analyze(request))
            .map_err(|_| "UCI engine thread stopped".to_string())
    }

    pub fn disconnect(mut self) {
        let _ = self.command_tx.send(WorkerCommand::Disconnect);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for UciSession {
    fn drop(&mut self) {
        let _ = self.command_tx.send(WorkerCommand::Disconnect);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn worker_loop(
    mut running: RunningEngine,
    command_rx: Receiver<WorkerCommand>,
    event_tx: Sender<UciEvent>,
) {
    while let Ok(command) = command_rx.recv() {
        match command {
            WorkerCommand::Analyze(request) => {
                let result = running.analyze(request);
                let _ = event_tx.send(match result {
                    Ok(analysis) => UciEvent::Analysis(analysis),
                    Err(err) => UciEvent::Error(err),
                });
            }
            WorkerCommand::Disconnect => break,
        }
    }

    running.quit();
    let _ = event_tx.send(UciEvent::Disconnected);
}

struct RunningEngine {
    child: Child,
    conn: Engine<BufReader<ChildStdout>, ChildStdin>,
    name: String,
}

impl RunningEngine {
    fn spawn(path: &Path) -> Result<Self, String> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("Failed to start engine: {err}"))?;

        let mut conn = Engine::from_process(&mut child, true)
            .map_err(|err| format!("Failed to capture engine I/O: {err}"))?;

        let name = uci_handshake(&mut conn)?;
        conn.is_ready()
            .map_err(|err| format!("Engine not ready: {err}"))?;

        Ok(Self { child, conn, name })
    }

    fn analyze(&mut self, request: AnalysisRequest) -> Result<AnalysisResult, String> {
        self.set_multipv(request.line_count)?;

        let position = Position::from_str(&format!("position fen {}", request.fen))
            .map_err(|err| format!("Invalid FEN: {err:?}"))?;
        self.conn
            .send(position)
            .map_err(|err| format!("Failed to send position: {err}"))?;

        let side_to_move = turn_color(&request.fen);
        let mut lines_at_depth: HashMap<usize, EngineLine> = HashMap::new();
        let mut best_depth = 0u32;

        let best_move = self
            .conn
            .go(
                &Go {
                    depth: Some(request.depth as usize),
                    ..Default::default()
                },
                |info: Info<'_>| {
                    let Some(depth) = info.depth.map(|d| d.depth as u32) else {
                        return;
                    };
                    if depth < best_depth {
                        return;
                    }
                    if depth > best_depth {
                        best_depth = depth;
                        lines_at_depth.clear();
                    }
                    let rank = info.multi_pv.unwrap_or(1);
                    if info.pv.is_empty() {
                        return;
                    }
                    let eval = info
                        .score
                        .as_ref()
                        .map(|score| eval_from_score(score, side_to_move));
                    lines_at_depth.insert(
                        rank,
                        EngineLine {
                            rank: rank as u32,
                            pv: info.pv.iter().map(|mv| mv.to_string()).collect(),
                            score: info.score.as_ref().map(format_score),
                            eval,
                        },
                    );
                },
            )
            .map_err(|err| format!("Analysis failed: {err}"))?;

        let _ = best_move;
        let mut lines: Vec<EngineLine> = lines_at_depth.into_values().collect();
        lines.sort_by_key(|line| line.rank);

        if lines.is_empty() {
            return Err("Engine returned no analysis lines".into());
        }

        Ok(AnalysisResult {
            depth: best_depth.max(request.depth),
            lines,
        })
    }

    fn set_multipv(&mut self, line_count: u32) -> Result<(), String> {
        let value = line_count.clamp(1, 10).to_string();
        self.conn
            .send(SetOption {
                name: "MultiPV".into(),
                value: Some(value.into()),
            })
            .map_err(|err| format!("Failed to set MultiPV: {err}"))?;
        self.conn
            .is_ready()
            .map_err(|err| format!("Engine not ready after MultiPV: {err}"))
    }

    fn quit(mut self) {
        let _ = self.conn.send(ruci::Quit);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn uci_handshake(conn: &mut Engine<BufReader<ChildStdout>, ChildStdin>) -> Result<String, String> {
    conn.send(ruci::Uci)
        .map_err(|err| format!("Failed to send uci: {err}"))?;

    let mut name = None::<String>;
    loop {
        let mut line = String::new();
        let bytes = conn
            .engine
            .read_line(&mut line)
            .map_err(|err| format!("Failed to read engine output: {err}"))?;
        if bytes == 0 {
            return Err("Engine closed connection during UCI handshake".into());
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "uciok" {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("id name ") {
            name = Some(rest.to_string());
        }
    }

    Ok(name.unwrap_or_else(|| "UCI engine".into()))
}

fn format_score(score: &engine::ScoreWithBound) -> String {
    let bound = match score.bound {
        Some(engine::ScoreBound::LowerBound) => " ≥",
        Some(engine::ScoreBound::UpperBound) => " ≤",
        None => "",
    };
    match score.kind {
        engine::Score::Centipawns(cp) => format!("{cp:+} cp{bound}"),
        engine::Score::MateIn(mate) => format!("M{mate}{bound}"),
    }
}

pub fn poll_uci_events(
    entity: gpui::Entity<crate::app::NoveltyApp>,
    engine_id: String,
    event_rx: Receiver<UciEvent>,
    cx: &mut gpui::App,
) {
    cx.spawn(async move |cx| {
        loop {
            match event_rx.try_recv() {
                Ok(event) => {
                    let done = matches!(event, UciEvent::Disconnected);
                    entity.update(cx, |app, cx| {
                        app.handle_uci_event(&engine_id, event, cx);
                    });
                    if done {
                        break;
                    }
                }
                Err(mpsc::TryRecvError::Empty) => {
                    cx.background_executor()
                        .timer(Duration::from_millis(50))
                        .await;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    let engine_id = engine_id.clone();
                    entity.update(cx, |app, cx| {
                        app.handle_uci_poll_closed(&engine_id, cx);
                    });
                    break;
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connects_to_reckless_if_installed() {
        let path = std::path::PathBuf::from(
            "/Users/anrey/.config/novelty/engines/reckless/v0.9.0/reckless-macos",
        );
        if !path.is_file() {
            return;
        }
        let (event_tx, _event_rx) = mpsc::channel();
        let (session, name) = UciSession::connect(path, event_tx).expect("reckless connect");
        assert!(name.contains("Reckless"));
        drop(session);
    }
}
