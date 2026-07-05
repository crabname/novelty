use gpui::*;

use crate::app::NoveltyApp;
use crate::panel_tabs::SidePanelTab;
use crate::ui::engine_panel::{EnginePanelState, EnginePanelTarget};

impl NoveltyApp {
    pub(crate) fn render_controls_panel(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let session = self.opening_tree_at(session_index);
        let session_id = session.map(|s| s.id).unwrap_or(0);
        let panel_tab = session
            .map(|s| s.side_panel_tab)
            .unwrap_or(SidePanelTab::Moves);
        let tabs = SidePanelTab::OPENING_TREE;
        let selected_index = panel_tab.index_in(tabs);

        let content = match panel_tab {
            SidePanelTab::Moves => self
                .render_next_moves_table(session_index, cx)
                .into_any_element(),
            SidePanelTab::Engine => {
                let analyzing = session.is_some_and(|s| s.engine.analyzing);
                let depth = session
                    .map(|s| s.engine.settings.depth)
                    .unwrap_or(14);
                let line_count = session
                    .map(|s| s.engine.settings.line_count)
                    .unwrap_or(3);
                let show_engine_lines = session
                    .map(|s| s.engine.settings.show_engine_lines)
                    .unwrap_or(true);
                let lines = session
                    .and_then(|s| s.engine.analysis.as_ref())
                    .map(|a| a.lines.as_slice())
                    .unwrap_or(&[]);
                let result_depth = session
                    .and_then(|s| s.engine.analysis.as_ref())
                    .map(|a| a.depth)
                    .unwrap_or(0);
                self.render_engine_panel(
                    &format!("opening-tree-engine-{session_id}"),
                    EnginePanelTarget::OpeningTree {
                        session_id,
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
            _ => div().into_any_element(),
        };

        self.render_right_panel(
            SharedString::from(format!("opening-tree-panel-{session_id}")),
            tabs,
            selected_index,
            move |this, index, cx| {
                if let Some(session) = this.opening_tree_at_mut(session_index) {
                    session.side_panel_tab = SidePanelTab::from_index(tabs, index);
                }
                cx.notify();
            },
            content,
            cx,
        )
    }
}
