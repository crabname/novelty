use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::separator::Separator;
use gpui_component::*;

use crate::panel_tabs::SidePanelTab;
use crate::app::NoveltyApp;
use crate::session::HistoryStep;
use crate::ui::engine_panel::{EnginePanelState, EnginePanelTarget};
use crate::ui::engine_pick::EnginePickTarget;

const ANALYSIS_SIDEBAR_WIDTH: f32 = 280.;

impl NoveltyApp {
    pub(crate) fn render_game_analysis(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_tab;

        h_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .items_stretch()
            .child(self.render_analysis_sidebar(window, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .items_stretch()
                    .child(self.render_analysis_history(active, cx))
                    .child(
                        div()
                            .id(SharedString::from(format!("analysis-board-{active}")))
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_handle.focus(window, cx);
                                }),
                            )
                            .child(self.render_analysis_board(active, cx)),
                    )
                    .child(self.render_analysis_panel(active, cx)),
            )
    }

    fn render_analysis_sidebar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.sidebar_collapsed {
            return v_flex()
                .id("analysis-sidebar-collapsed")
                .flex_shrink_0()
                .w(px(40.))
                .h_full()
                .min_h_0()
                .border_r_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().sidebar)
                .items_center()
                .pt_2()
                .child(self.render_sidebar_toggle(cx));
        }

        let session = self.active_game_analysis().expect("analysis tab");
        let tab_id = session.id;
        let status = session.status.clone();
        let history_index = session.history_index;
        let move_total = session.history.len().saturating_sub(1);
        let pgn_input = session.pgn_input.clone();
        let selected_engine = session
            .engine
            .selected_engine_id
            .clone()
            .unwrap_or_default();
        let engines = self.engines.clone();
        let opening_label = session.opening_label();

        v_flex()
            .id("analysis-sidebar")
            .flex_shrink_0()
            .w(px(ANALYSIS_SIDEBAR_WIDTH))
            .h_full()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .child(
                h_flex()
                    .px_3()
                    .py_3()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        Label::new("Analysis")
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(self.render_sidebar_toggle(cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .gap_3()
                    .px_3()
                    .pb_3()
                    .overflow_y_scrollbar()
                    .child(
                        GroupBox::new()
                            .title("PGN")
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        Label::new("Paste a game or movetext")
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(Input::new(&pgn_input).h(px(140.)).w_full()),
                            ),
                    )
                    .child(
                        GroupBox::new()
                            .title("Opening")
                            .child(
                                Label::new(opening_label)
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .child(
                        GroupBox::new()
                            .title("Engine")
                            .child(
                                v_flex()
                                    .gap_2()
                                    .children(if engines.is_empty() {
                                        vec![Label::new("Add an engine in the Engine tab")
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .into_any_element()]
                                    } else {
                                        engines
                                            .iter()
                                            .map(|engine| {
                                                self.render_engine_pick_card(
                                                    &format!(
                                                        "analysis-sidebar-engine-{}",
                                                        engine.id
                                                    ),
                                                    engine,
                                                    selected_engine == engine.id,
                                                    EnginePickTarget::GameAnalysis(tab_id),
                                                    cx,
                                                )
                                                .into_any_element()
                                            })
                                            .collect()
                                    }),
                            ),
                    )
                    .child(
                        GroupBox::new()
                            .title("Navigation")
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "analysis-back-{tab_id}"
                                        )))
                                        .label("←")
                                        .disabled(history_index == 0)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(session) =
                                                this.game_analysis_by_id_mut(tab_id)
                                            {
                                                session.go_back(cx);
                                            }
                                            this.refresh_analysis_if_engine_selected(tab_id, cx);
                                            this.refresh_explorer_if_needed(tab_id, cx);
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "analysis-fwd-{tab_id}"
                                        )))
                                        .label("→")
                                        .disabled(history_index >= move_total)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(session) =
                                                this.game_analysis_by_id_mut(tab_id)
                                            {
                                                session.go_forward(cx);
                                            }
                                            this.refresh_analysis_if_engine_selected(tab_id, cx);
                                            this.refresh_explorer_if_needed(tab_id, cx);
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "analysis-start-{tab_id}"
                                        )))
                                        .label("Start")
                                        .disabled(history_index == 0)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(session) =
                                                this.game_analysis_by_id_mut(tab_id)
                                            {
                                                session.go_to_history(0, cx);
                                            }
                                            this.refresh_analysis_if_engine_selected(tab_id, cx);
                                            this.refresh_explorer_if_needed(tab_id, cx);
                                            cx.notify();
                                        })),
                                    ),
                            ),
                    ),
            )
            .child(Separator::horizontal())
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(status),
            )
    }

    fn render_analysis_board(&self, tab_index: usize, _cx: &App) -> impl IntoElement {
        let board = self
            .game_analysis_at(tab_index)
            .map(|session| session.board.clone());

        div()
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .when_some(board, |this, board| this.child(board))
    }

    fn render_analysis_history(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .flex_shrink_0()
            .w(px(220.))
            .h_full()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(self.render_analysis_history_table(session_index, cx))
    }

    fn analysis_history_cell(
        &self,
        cx: &mut Context<Self>,
        session_index: usize,
        index: usize,
        label: SharedString,
    ) -> impl IntoElement {
        let tab_id = self.tabs[session_index].id();
        let selected = self
            .game_analysis_at(session_index)
            .is_some_and(|session| session.history_index == index);
        div()
            .id(SharedString::from(format!("analysis-hist-{session_index}-{index}")))
            .min_w(px(44.))
            .px_2()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .when(selected, |el| el.bg(cx.theme().accent.opacity(0.25)))
            .when(!selected, |el| el.hover(|s| s.bg(cx.theme().muted)))
            .text_sm()
            .text_align(TextAlign::Center)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if let Some(session) = this.game_analysis_at_mut(session_index) {
                        session.go_to_history(index, cx);
                    }
                    this.refresh_analysis_if_engine_selected(tab_id, cx);
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                }),
            )
            .child(label)
    }

    fn render_analysis_history_table(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let history: &[HistoryStep] = self
            .game_analysis_at(session_index)
            .map(|session| session.history.as_slice())
            .unwrap_or(&[]);
        let move_rows = history.len().saturating_sub(1).div_ceil(2);
        let header = h_flex()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().muted.opacity(0.35))
            .child(
                div()
                    .w(px(36.))
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("#"),
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("White"),
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Black"),
            );

        let start_row = h_flex()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .py_0p5()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().w(px(36.)).text_xs().text_color(cx.theme().muted_foreground))
            .child(
                div()
                    .flex_1()
                    .child(self.analysis_history_cell(cx, session_index, 0, "Start".into())),
            )
            .child(div().flex_1());

        let mut rows = vec![start_row.into_any_element()];
        for move_number in 1..=move_rows {
            let white_index = move_number * 2 - 1;
            let black_index = move_number * 2;
            let white_cell = if white_index < history.len() {
                self.analysis_history_cell(
                    cx,
                    session_index,
                    white_index,
                    history[white_index]
                        .san
                        .clone()
                        .unwrap_or_default()
                        .into(),
                )
                .into_any_element()
            } else {
                div().flex_1().into_any_element()
            };
            let black_cell = if black_index < history.len() {
                self.analysis_history_cell(
                    cx,
                    session_index,
                    black_index,
                    history[black_index]
                        .san
                        .clone()
                        .unwrap_or_default()
                        .into(),
                )
                .into_any_element()
            } else {
                div().flex_1().into_any_element()
            };
            rows.push(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .px_2()
                    .py_0p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .w(px(36.))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{move_number}.")),
                    )
                    .child(div().flex_1().child(white_cell))
                    .child(div().flex_1().child(black_cell))
                    .into_any_element(),
            );
        }

        v_flex()
            .size_full()
            .min_h_0()
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .child(header)
                    .children(rows),
            )
    }

    fn render_analysis_panel(
        &mut self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab_id = self.tabs[session_index].id();
        if let Some(session) = self.game_analysis_at(session_index) {
            self.refresh_explorer_if_needed(session.id, cx);
        }
        let panel_tab = self
            .game_analysis_at(session_index)
            .map(|s| s.side_panel_tab)
            .unwrap_or(SidePanelTab::Engine);
        let tabs = SidePanelTab::GAME_ANALYSIS;
        let selected_index = panel_tab.index_in(tabs);

        let content = match panel_tab {
            SidePanelTab::Engine => {
                let analyzing = self
                    .game_analysis_at(session_index)
                    .is_some_and(|s| s.engine.analyzing);
                let depth = self
                    .game_analysis_at(session_index)
                    .map(|s| s.engine.settings.depth)
                    .unwrap_or(16);
                let line_count = self
                    .game_analysis_at(session_index)
                    .map(|s| s.engine.settings.line_count)
                    .unwrap_or(3);
                let show_engine_lines = self
                    .game_analysis_at(session_index)
                    .map(|s| s.engine.settings.show_engine_lines)
                    .unwrap_or(true);
                let lines = self
                    .game_analysis_at(session_index)
                    .and_then(|s| s.engine.analysis.as_ref())
                    .map(|a| a.lines.as_slice())
                    .unwrap_or(&[]);
                let result_depth = self
                    .game_analysis_at(session_index)
                    .and_then(|s| s.engine.analysis.as_ref())
                    .map(|a| a.depth)
                    .unwrap_or(0);
                self.render_engine_panel(
                    &format!("analysis-engine-{tab_id}"),
                    EnginePanelTarget::GameAnalysis {
                        tab_id,
                        session_index,
                    },
                    EnginePanelState {
                        analyzing,
                        depth,
                        line_count,
                        show_engine_lines,
                        lines,
                        result_depth,
                    },
                    cx,
                )
                .into_any_element()
            }
            SidePanelTab::Explorer => self
                .render_explorer_moves_table(session_index, cx)
                .into_any_element(),
            SidePanelTab::Game => self
                .render_analysis_game_tab(session_index, cx)
                .into_any_element(),
            _ => div().into_any_element(),
        };

        let tab_id_for_panel = tab_id;
        self.render_right_panel(
            SharedString::from(format!("analysis-panel-{tab_id_for_panel}")),
            tabs,
            selected_index,
            move |this, index, cx| {
                let tab_id = this
                    .game_analysis_at(session_index)
                    .map(|session| session.id)
                    .unwrap_or(0);
                if let Some(session) = this.game_analysis_at_mut(session_index) {
                    session.side_panel_tab = SidePanelTab::from_index(tabs, index);
                }
                if SidePanelTab::from_index(tabs, index) == SidePanelTab::Explorer {
                    this.refresh_explorer_if_needed(tab_id, cx);
                }
                cx.notify();
            },
            content,
            cx,
        )
    }

    fn render_analysis_game_tab(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let headers = self
            .game_analysis_at(session_index)
            .map(|s| s.headers.clone())
            .unwrap_or_default();

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_y_scrollbar()
            .p_4()
            .child(if headers.is_empty() {
                Label::new("Load a PGN to see game tags")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .into_any_element()
            } else {
                v_flex()
                    .gap_1()
                    .children(headers.iter().map(|(tag, value)| {
                        div()
                            .text_sm()
                            .child(format!("[{tag}] {value}"))
                            .into_any_element()
                    }))
                    .into_any_element()
            })
    }
}
