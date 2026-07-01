//! FIDE performance rating (`getPerformanceDetails` / `DP_TABLE`).

use crate::fetch::PlayerColor;
use crate::graph::PositionDetails;

#[derive(Clone, Debug)]
pub struct PerformanceDetails {
    pub performance_rating: Option<i32>,
    pub results: String,
    pub average_opponent_elo: Option<u32>,
    pub score: String,
    pub win_percent: f32,
}

/// FIDE DP table: score % (rounded) → rating change vs average opponent.
fn dp_rating_change(score_percent: f32) -> i32 {
    const TABLE: [i32; 101] = [
        -800, -677, -589, -538, -501, -470, -444, -422, -401, -383, -366, -351, -336, -322,
        -309, -296, -284, -273, -262, -251, -240, -230, -220, -211, -202, -193, -184, -175,
        -166, -158, -149, -141, -133, -125, -117, -110, -102, -95, -87, -80, -72, -65, -57,
        -50, -43, -36, -29, -21, -14, -7, 0, 7, 14, 21, 29, 36, 43, 50, 57, 65, 72, 80, 87,
        95, 102, 110, 117, 125, 133, 141, 149, 158, 166, 175, 184, 193, 202, 211, 220, 230,
        240, 251, 262, 273, 284, 296, 309, 322, 336, 351, 366, 383, 401, 422, 444, 470, 501,
        538, 589, 677, 800,
    ];
    let idx = score_percent.round().clamp(0., 100.) as usize;
    TABLE[idx]
}

pub fn simplify_count(count: u32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.)
    } else if count >= 10_000 {
        format!("{}k", (count as f64 / 1000.).round() as u32)
    } else {
        count.to_string()
    }
}

pub fn performance_details(
    details: &PositionDetails,
    player_color: PlayerColor,
    has_player: bool,
) -> PerformanceDetails {
    let total = details.total();
    let white = details.white_wins;
    let draws = details.draws;
    let black = details.black_wins;

    let (player_wins, player_losses) = match player_color {
        PlayerColor::White => (white, black),
        PlayerColor::Black => (black, white),
    };

    let score = player_wins as f32 + draws as f32 / 2.;
    let score_percent = if total > 0 {
        score * 100. / total as f32
    } else {
        0.
    };

    let average_opponent_elo = details.average_opponent_elo();
    let performance_rating = if has_player {
        average_opponent_elo.map(|elo| elo as i32 + dp_rating_change(score_percent))
    } else {
        None
    };

    let color_label = match player_color {
        PlayerColor::White => "white",
        PlayerColor::Black => "black",
    };

    let score_label = if score_percent.fract() < f32::EPSILON {
        format!("{:.0}% for {color_label}", score_percent)
    } else {
        format!("{score_percent:.1}% for {color_label}")
    };

    PerformanceDetails {
        performance_rating,
        results: format!(
            "+{}-{}={}",
            simplify_count(player_wins),
            simplify_count(player_losses),
            simplify_count(draws)
        ),
        average_opponent_elo,
        score: score_label,
        win_percent: if total > 0 {
            player_wins as f32 * 100. / total as f32
        } else {
            0.
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dp_table_endpoints() {
        assert_eq!(dp_rating_change(100.), 800);
        assert_eq!(dp_rating_change(50.), 0);
        assert_eq!(dp_rating_change(0.), -800);
    }

    #[test]
    fn performance_from_position_details() {
        let mut details = PositionDetails::default();
        details.white_wins = 6;
        details.black_wins = 2;
        details.draws = 2;
        details.total_opponent_elo = 1800 * 10;
        details.opponent_elo_games = 10;

        let perf = performance_details(&details, PlayerColor::White, true);
        assert_eq!(perf.results, "+6-2=2");
        assert_eq!(perf.average_opponent_elo, Some(1800));
        assert!(perf.performance_rating.is_some());
        assert_eq!(perf.win_percent, 60.);
    }
}
