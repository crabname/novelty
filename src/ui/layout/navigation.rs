use gpui::*;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::*;

use crate::app::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn render_navigation_group<B, F, S>(
        &self,
        back_id: impl Into<SharedString>,
        forward_id: impl Into<SharedString>,
        start_id: impl Into<SharedString>,
        back_disabled: bool,
        forward_disabled: bool,
        start_disabled: bool,
        on_back: B,
        on_forward: F,
        on_start: S,
        cx: &mut Context<Self>,
    ) -> impl IntoElement
    where
        B: Fn(&mut Self, &mut Context<Self>) + Clone + 'static,
        F: Fn(&mut Self, &mut Context<Self>) + Clone + 'static,
        S: Fn(&mut Self, &mut Context<Self>) + Clone + 'static,
    {
        GroupBox::new()
            .title("Navigation")
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(back_id.into())
                            .label("←")
                            .disabled(back_disabled)
                            .on_click(cx.listener(move |this, _, _, cx| on_back(this, cx))),
                    )
                    .child(
                        Button::new(forward_id.into())
                            .label("→")
                            .disabled(forward_disabled)
                            .on_click(cx.listener(move |this, _, _, cx| on_forward(this, cx))),
                    )
                    .child(
                        Button::new(start_id.into())
                            .label("Start")
                            .disabled(start_disabled)
                            .on_click(cx.listener(move |this, _, _, cx| on_start(this, cx))),
                    ),
            )
    }
}
