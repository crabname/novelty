//! Lichess Opening Explorer (https://explorer.lichess.org).

use gpui::App;

use serde::Deserialize;

use crate::graph::PositionDetails;

const EXPLORER_URL: &str = "https://explorer.lichess.org/lichess";

/// Domain sessions implement this to share explorer fetch and UI.
pub trait ExplorerHost {
    fn explorer_fen(&self) -> &str;
    fn explorer_state(&self) -> &ExplorerState;
    fn explorer_state_mut(&mut self) -> &mut ExplorerState;
    fn play_explorer_san(&mut self, san: &str, cx: &mut App);
}

impl ExplorerState {
    pub fn needs_fetch(&self, fen: &str) -> bool {
        !self.loading && self.fen != fen
    }

    pub fn begin_fetch(&mut self, request_id: u64) {
        self.loading = true;
        self.error = None;
        self.request_id = request_id;
    }

    pub fn apply_moves(&mut self, request_id: u64, fen: String, moves: Vec<ExplorerMove>) {
        if self.request_id != request_id {
            return;
        }
        self.loading = false;
        self.fen = fen;
        self.moves = moves;
        self.error = None;
    }

    pub fn set_error(&mut self, request_id: u64, message: String) {
        if self.request_id != request_id {
            return;
        }
        self.loading = false;
        self.moves.clear();
        self.error = Some(message);
    }
}

#[derive(Clone, Debug)]
pub struct ExplorerMove {
    pub san: String,
    pub white: u32,
    pub black: u32,
    pub draws: u32,
}

impl ExplorerMove {
    pub fn total(&self) -> u32 {
        self.white + self.black + self.draws
    }

    pub fn position_details(&self) -> PositionDetails {
        PositionDetails {
            white_wins: self.white,
            black_wins: self.black,
            draws: self.draws,
            ..Default::default()
        }
    }
}

pub fn explorer_grand_total(moves: &[ExplorerMove]) -> u32 {
    moves.iter().map(ExplorerMove::total).sum()
}

pub fn aggregate_explorer_details(moves: &[ExplorerMove]) -> PositionDetails {
    let mut details = PositionDetails::default();
    for mv in moves {
        details.white_wins += mv.white;
        details.black_wins += mv.black;
        details.draws += mv.draws;
    }
    details
}

/// Share of all games at this position; raw count when the sample is small.
pub fn explorer_move_share_label(move_total: u32, grand_total: u32) -> String {
    if grand_total == 0 {
        return "—".into();
    }
    if move_total > 100 {
        format!("{:.0}%", move_total as f32 * 100. / grand_total as f32)
    } else {
        move_total.to_string()
    }
}

#[derive(Clone, Debug, Default)]
pub struct ExplorerState {
    pub loading: bool,
    pub fen: String,
    pub moves: Vec<ExplorerMove>,
    pub error: Option<String>,
    pub request_id: u64,
}

#[derive(Deserialize)]
struct ExplorerResponse {
    moves: Vec<ExplorerMoveJson>,
}

#[derive(Deserialize)]
struct ExplorerMoveJson {
    san: String,
    white: u32,
    black: u32,
    draws: u32,
}

pub fn fetch_lichess_explorer(fen: &str, token: Option<&str>) -> Result<Vec<ExplorerMove>, String> {
    let client = crate::fetch::http_client();
    let url = format!(
        "{EXPLORER_URL}?fen={}&moves=12",
        urlencoding::encode(fen)
    );
    let mut request = client.get(url);
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = request
        .send()
        .map_err(|err| format!("Opening Explorer request failed: {err}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let message = if status.as_u16() == 401 {
            "Opening Explorer requires Lichess login. Connect your account in Settings."
        } else {
            return Err(format!("Opening Explorer returned HTTP {status}"));
        };
        return Err(message.into());
    }
    let body: ExplorerResponse = response
        .json()
        .map_err(|err| format!("Opening Explorer response invalid: {err}"))?;
    Ok(body
        .moves
        .into_iter()
        .map(|mv| ExplorerMove {
            san: mv.san,
            white: mv.white,
            black: mv.black,
            draws: mv.draws,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_label_uses_percent_above_threshold() {
        assert_eq!(explorer_move_share_label(150, 1000), "15%");
        assert_eq!(explorer_move_share_label(100, 1000), "100");
        assert_eq!(explorer_move_share_label(50, 200), "50");
    }

    #[test]
    fn aggregate_sums_all_moves() {
        let moves = vec![
            ExplorerMove {
                san: "e4".into(),
                white: 10,
                black: 5,
                draws: 2,
            },
            ExplorerMove {
                san: "d4".into(),
                white: 8,
                black: 4,
                draws: 1,
            },
        ];
        let details = aggregate_explorer_details(&moves);
        assert_eq!(details.white_wins, 18);
        assert_eq!(details.black_wins, 9);
        assert_eq!(details.draws, 3);
        assert_eq!(explorer_grand_total(&moves), 30);
    }
}
