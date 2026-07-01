use gpui::*;
use gpui_component::button::Button;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::Sizable;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::tab::TabKind;

impl NoveltyApp {
    pub(crate) fn render_home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("novelty-home")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scrollbar()
            .p_6()
            .gap_6()
            .child(self.render_home_section(
                "Game modes",
                "Open a board workspace in a new tab",
                TabKind::game_modes(),
                cx,
            ))
            .child(self.render_home_section(
                "Tools",
                "Databases, engines, and settings",
                TabKind::tool_modes(),
                cx,
            ))
    }

    fn render_home_section(
        &self,
        title: &'static str,
        subtitle: &'static str,
        kinds: &'static [TabKind],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(title)
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        Label::new(subtitle)
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child({
                let mut tiles = Vec::new();
                for &kind in kinds {
                    tiles.push(self.render_mode_tile(kind, cx).into_any_element());
                }
                div().flex().flex_wrap().gap_4().children(tiles)
            })
    }

    fn render_mode_tile(&self, kind: TabKind, cx: &mut Context<Self>) -> impl IntoElement {
        let icon = kind.icon();
        let title = kind.label();
        let description = kind.description();
        let implemented = kind.is_implemented();

        div()
            .id(SharedString::from(format!("home-tile-{}", kind.tile_id())))
            .w(px(200.))
            .min_h(px(220.))
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().group_box)
            .hover(|s| s.bg(cx.theme().accent.opacity(0.08)))
            .cursor_pointer()
            .flex()
            .flex_col()
            .items_center()
            .justify_between()
            .gap_3()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    this.open_mode(kind, window, cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .flex_1()
                    .child(
                        Icon::new(icon)
                            .text_color(cx.theme().foreground)
                            .with_size(gpui_component::Size::Size(px(48.))),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                Label::new(title)
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_align(TextAlign::Center),
                            )
                            .child(
                                Label::new(description)
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .text_align(TextAlign::Center),
                            ),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("home-open-{}", kind.tile_id())))
                    .label(if implemented { "Open" } else { "Coming soon" })
                    .disabled(!implemented)
                    .w_full()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if kind.is_implemented() {
                            this.open_mode(kind, window, cx);
                        }
                    })),
            )
    }
}
