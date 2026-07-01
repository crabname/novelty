use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::session::ControlPanelTab;
use crate::ui::engine_panel::EnginePanelTarget;

impl NoveltyApp {
    pub(crate) fn render_controls_panel(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let session = self.opening_tree_at(session_index);
        let tab_index = session
            .map(|session| session.control_tab.index())
            .unwrap_or(0);
        let session_id = session.map(|s| s.id).unwrap_or(0);
        let panel_tab = session
            .map(|s| s.control_tab)
            .unwrap_or_default();

        v_flex()
            .flex_shrink_0()
            .w(px(380.))
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                TabBar::new(SharedString::from(format!(
                    "opening-tree-panel-tabs-{session_id}"
                )))
                .flex_shrink_0()
                .px_2()
                .pt_2()
                .selected_index(tab_index)
                .on_click(cx.listener(move |this, ix: &usize, _, cx| {
                    if let Some(session) = this.opening_tree_at_mut(session_index) {
                        session.control_tab = ControlPanelTab::from_index(*ix);
                    }
                    cx.notify();
                }))
                .child(Tab::new().label("Moves"))
                .child(Tab::new().label("Engine")),
            )
            .child(
                div()
                    .id("control-panel-content")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(match panel_tab {
                        ControlPanelTab::NextMoves => self
                            .render_next_moves_table(session_index, cx)
                            .into_any_element(),
                        ControlPanelTab::Engine => {
                            let analyzing = session.is_some_and(|s| s.analyzing);
                            let depth = session.map(|s| s.settings.depth).unwrap_or(14);
                            let line_count = session.map(|s| s.settings.line_count).unwrap_or(3);
                            let show_engine_lines = session
                                .map(|s| s.settings.show_engine_lines)
                                .unwrap_or(true);
                            let lines = session
                                .and_then(|s| s.analysis.as_ref())
                                .map(|a| a.lines.as_slice())
                                .unwrap_or(&[]);
                            let result_depth = session
                                .and_then(|s| s.analysis.as_ref())
                                .map(|a| a.depth)
                                .unwrap_or(0);
                            self.render_engine_panel(
                                &format!("opening-tree-engine-{session_id}"),
                                EnginePanelTarget::OpeningTree {
                                    session_id,
                                    session_index,
                                },
                                analyzing,
                                depth,
                                line_count,
                                show_engine_lines,
                                lines,
                                result_depth,
                                cx,
                            )
                            .into_any_element()
                        }
                    }),
            )
    }
}
