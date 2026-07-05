use gpui::*;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::select::Select;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::fetch::{PlayerColor, Site};

impl NoveltyApp {
    pub(super) fn render_repertoire_import_section(
        &self,
        cx: &mut Context<Self>,
        opening_label: SharedString,
        profile_input: Entity<InputState>,
        import_site: Site,
        import_depth: u8,
        import_loading: bool,
        player_color: PlayerColor,
    ) -> impl IntoElement {
        GroupBox::new()
            .title("Import from profile")
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        Label::new("At current board position")
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(Label::new(opening_label).text_sm())
                    .child(
                        Label::new(format!(
                            "Games as {} · continuations added as variations",
                            player_color.orientation_value()
                        ))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground),
                    )
                    .child(Input::new(&profile_input))
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Period")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                Select::new(&self.period_select)
                                    .w_full()
                                    .disabled(import_loading),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Time controls")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                self.render_time_control_checkbox(
                                    cx,
                                    "repertoire-tc-bullet",
                                    "Bullet",
                                    self.time_controls.bullet,
                                    import_loading,
                                    |filter| &mut filter.bullet,
                                ),
                            )
                            .child(
                                self.render_time_control_checkbox(
                                    cx,
                                    "repertoire-tc-blitz",
                                    "Blitz",
                                    self.time_controls.blitz,
                                    import_loading,
                                    |filter| &mut filter.blitz,
                                ),
                            )
                            .child(
                                self.render_time_control_checkbox(
                                    cx,
                                    "repertoire-tc-rapid",
                                    "Rapid",
                                    self.time_controls.rapid,
                                    import_loading,
                                    |filter| &mut filter.rapid,
                                ),
                            )
                            .child(
                                self.render_time_control_checkbox(
                                    cx,
                                    "repertoire-tc-classical",
                                    "Classical",
                                    self.time_controls.classical,
                                    import_loading,
                                    |filter| &mut filter.classical,
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("repertoire-site-lichess")
                                    .label("Lichess")
                                    .selected(import_site == Site::Lichess)
                                    .disabled(import_loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(session) = this.active_repertoire_mut() {
                                            session.import_site = Site::Lichess;
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("repertoire-site-chesscom")
                                    .label("Chess.com")
                                    .selected(import_site == Site::ChessCom)
                                    .disabled(import_loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(session) = this.active_repertoire_mut() {
                                            session.import_site = Site::ChessCom;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Label::new("Depth (plies from here)")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                Button::new("repertoire-depth-dec")
                                    .label("−")
                                    .disabled(import_loading || import_depth <= 2)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(session) = this.active_repertoire_mut() {
                                            session.import_depth = session
                                                .import_depth
                                                .saturating_sub(1)
                                                .max(2);
                                        }
                                        cx.notify();
                                    })),
                            )
                            .child(Label::new(import_depth.to_string()).text_sm())
                            .child(
                                Button::new("repertoire-depth-inc")
                                    .label("+")
                                    .disabled(import_loading || import_depth >= 20)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(session) = this.active_repertoire_mut() {
                                            session.import_depth = session
                                                .import_depth
                                                .saturating_add(1)
                                                .min(20);
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        Button::new("repertoire-import-profile")
                            .label(if import_loading {
                                "Loading…"
                            } else {
                                "Import lines from profile"
                            })
                            .disabled(import_loading)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.import_repertoire_from_profile(window, cx);
                            })),
                    ),
            )
    }
}
