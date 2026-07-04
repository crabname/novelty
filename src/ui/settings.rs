use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::*;

use crate::app::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("novelty-settings")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scrollbar()
            .p_6()
            .gap_4()
            .child(self.render_settings_header(cx))
            .child(self.render_lichess_settings(cx))
    }

    fn render_settings_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                Label::new("Settings")
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                Label::new("Account connections and app preferences.")
                    .text_sm()
                    .text_color(cx.theme().muted_foreground),
            )
    }

    fn render_lichess_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let logged_in = self.lichess_logged_in();
        let lichess_user = self.lichess_username().unwrap_or_default();
        let auth_status = self.lichess_auth_status();

        GroupBox::new()
            .title("Lichess")
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        Label::new(
                            "Sign in to load your own games, including private ones. \
                             OAuth opens in your browser.",
                        )
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Username")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(Input::new(&self.username)),
                    )
                    .when(logged_in, |group| {
                        group.child(
                            Label::new(format!("Signed in as {lichess_user}"))
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("settings-lichess-login")
                                    .label("Login")
                                    .disabled(logged_in)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.login_lichess_browser(cx);
                                    })),
                            )
                            .child(
                                Button::new("settings-lichess-logout")
                                    .label("Logout")
                                    .disabled(!logged_in)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.logout_lichess(cx);
                                    })),
                            ),
                    )
                    .when(!auth_status.is_empty(), |group| {
                        group.child(
                            Label::new(auth_status)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
    }
}
