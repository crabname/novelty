use gpui::*;
use gpui_component::tab::{Tab, TabBar};
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::panel_tabs::SidePanelTab;

pub const RIGHT_PANEL_WIDTH: f32 = 380.;

impl NoveltyApp {
    pub(crate) fn render_right_panel<F>(
        &self,
        panel_id: impl Into<SharedString>,
        tabs: &[SidePanelTab],
        selected_index: usize,
        on_tab_click: F,
        content: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        F: Fn(&mut Self, usize, &mut Context<Self>) + Clone + 'static,
    {
        let panel_id = panel_id.into();
        v_flex()
            .flex_shrink_0()
            .w(px(RIGHT_PANEL_WIDTH))
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                TabBar::new(SharedString::from(format!("{panel_id}-tabs")))
                    .flex_shrink_0()
                    .px_2()
                    .pt_2()
                    .selected_index(selected_index)
                    .on_click(cx.listener(move |this, ix: &usize, _, cx| {
                        on_tab_click(this, *ix, cx);
                    }))
                    .children(tabs.iter().map(|tab| Tab::new().label(tab.label()))),
            )
            .child(
                div()
                    .id(SharedString::from(format!("{panel_id}-content")))
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(content),
            )
    }
}
