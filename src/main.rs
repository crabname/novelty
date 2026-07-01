//! Novelty — opening repertoire viewer for Lichess and Chess.com profiles.
//!
//! Loads recent games for a username, aggregates move frequencies, and draws arrows
//! on the board (thicker green = more common). Play only repertoire continuations.
//!
//! Run: `cargo run`

mod analysis_session;
mod app;
mod constants;
mod eval_display;
mod engine_catalog;
mod engine_shapes;
mod engine_uci;
mod engines;
mod fetch;
mod graph;
mod performance;
mod pgn;
mod profiles;
mod session;
mod tab;
mod ui;

use gpui::*;
use gpui_component::input::InputState;
use gpui_component::searchable_list::SearchableVec;
use gpui_component::select::SelectState;
use gpui_component::*;
use gpui_component_assets::Assets;

use app::NoveltyApp;
use fetch::LoadPeriod;
use profiles::load_profiles;

fn main() {
    let app = gpui_platform::application()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    app.run(move |cx| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1280.), px(820.)), cx)),
            titlebar: Some(TitleBar::title_bar_options()),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let profile_history = load_profiles();
                let default_username = profile_history
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "DrNykterstein".into());
                let username = cx.new(|cx| {
                    InputState::new(window, cx)
                        .default_value(default_username)
                        .placeholder("Lichess / Chess.com username")
                });
                let period_select = cx.new(|cx| {
                    SelectState::new(
                        LoadPeriod::ALL.to_vec(),
                        Some(IndexPath::default()),
                        window,
                        cx,
                    )
                });
                let profile_select = cx.new(|cx| {
                    SelectState::new(
                        SearchableVec::new(profile_history.clone()),
                        None,
                        window,
                        cx,
                    )
                    .searchable(true)
                });
                let shell = cx.new(|cx| {
                    NoveltyApp::new(
                        username,
                        period_select,
                        profile_select,
                        profile_history,
                        window,
                        cx,
                    )
                });
                cx.new(|cx| Root::new(shell, window, cx))
            })
            .expect("failed to open window");
        })
        .detach();
    });
}
