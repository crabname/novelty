use gpui::*;
use gpui_component::label::Label;
use gpui_component::Sizable;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::tab::TabKind;

impl NoveltyApp {
    pub(crate) fn render_stub(&self, kind: TabKind, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id(SharedString::from(format!("novelty-stub-{}", kind.tile_id())))
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                Icon::new(kind.icon())
                    .text_color(cx.theme().muted_foreground)
                    .with_size(gpui_component::Size::Size(px(64.))),
            )
            .child(
                Label::new(kind.label())
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                Label::new("Coming soon")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                Label::new(kind.description())
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .text_align(TextAlign::Center)
                    .max_w(px(420.)),
            )
    }
}
