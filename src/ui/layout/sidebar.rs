use gpui::*;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::separator::Separator;
use gpui_component::sidebar::SidebarToggleButton;
use gpui_component::*;

use crate::app::NoveltyApp;

pub const SIDEBAR_COLLAPSED_WIDTH: f32 = 40.;

impl NoveltyApp {
    pub(crate) fn render_sidebar_toggle(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        SidebarToggleButton::new()
            .collapsed(self.sidebar_collapsed)
            .on_click(cx.listener(|this, _, _, cx| {
                this.sidebar_collapsed = !this.sidebar_collapsed;
                cx.notify();
            }))
    }

    pub(crate) fn render_collapsed_sidebar(
        &self,
        id: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .id(id.into())
            .flex_shrink_0()
            .w(px(SIDEBAR_COLLAPSED_WIDTH))
            .h_full()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().sidebar)
            .items_center()
            .pt_2()
            .child(self.render_sidebar_toggle(cx))
            .into_any_element()
    }

    pub(crate) fn render_expanded_sidebar(
        &self,
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        width: f32,
        status: impl Into<SharedString>,
        scroll_body: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let status = status.into();
        v_flex()
            .id(id.into())
            .flex_shrink_0()
            .w(px(width))
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
                        Label::new(title)
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(self.render_sidebar_toggle(cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .px_3()
                    .pb_3()
                    .overflow_y_scrollbar()
                    .child(scroll_body),
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
            .into_any_element()
    }
}
