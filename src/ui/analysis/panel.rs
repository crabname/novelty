use gpui::*;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::panel_tabs::SidePanelTab;
use crate::ui::engine_panel::{EnginePanelState, EnginePanelTarget};

impl NoveltyApp {
    pub(super) fn render_analysis_panel(
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
