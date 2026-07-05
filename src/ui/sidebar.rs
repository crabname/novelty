use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::select::Select;
use gpui_component::switch::Switch;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::fetch::{PlayerColor, Site, TimeControlFilter};
use crate::ui::engine_pick::EnginePickTarget;

const SIDEBAR_WIDTH: f32 = 280.;

impl NoveltyApp {
    pub(crate) fn render_time_control_checkbox(
        &self,
        cx: &mut Context<Self>,
        id: &'static str,
        label: &'static str,
        checked: bool,
        disabled: bool,
        field: fn(&mut TimeControlFilter) -> &mut bool,
    ) -> impl IntoElement {
        Checkbox::new(id)
            .label(label)
            .checked(checked)
            .disabled(disabled)
            .on_click(cx.listener(move |this, is_checked, _, cx| {
                *field(&mut this.time_controls) = *is_checked;
                cx.notify();
            }))
    }

    pub(crate) fn render_sidebar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.sidebar_collapsed {
            return self.render_collapsed_sidebar("novelty-sidebar-collapsed", cx);
        }

        let session = self.active_opening_tree().expect("opening tree tab");
        let session_id = session.id;
        let loading = session.loading;
        let status = session.status.clone();
        let history_index = session.history_index;
        let next_move_count = session.next_move_count;
        let opening_label = session.opening_label();
        let selected_engine = session
            .engine
            .selected_engine_id
            .clone()
            .unwrap_or_default();
        let engines = self.engines.clone();

        let scroll_body = v_flex()
            .gap_3()
            .child(
                GroupBox::new()
                    .title("Profile")
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Label::new("Loads into the active profile tab")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(Input::new(&self.username))
                            .when(!self.profile_history.is_empty(), |this| {
                                this.child(
                                    Select::new(&self.profile_select)
                                        .w_full()
                                        .placeholder("Recent profiles"),
                                )
                            }),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Source")
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("site-lichess")
                                    .label("Lichess")
                                    .selected(self.site == Site::Lichess)
                                    .disabled(loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.site = Site::Lichess;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("site-chesscom")
                                    .label("Chess.com")
                                    .selected(self.site == Site::ChessCom)
                                    .disabled(loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.site = Site::ChessCom;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Filters")
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Switch::new("color-white")
                                    .label("As White")
                                    .checked(self.color == PlayerColor::White)
                                    .disabled(loading)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.color = if *checked {
                                            PlayerColor::White
                                        } else {
                                            PlayerColor::Black
                                        };
                                        cx.notify();
                                    })),
                            )
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
                                            .disabled(loading),
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
                                            "tc-bullet",
                                            "Bullet",
                                            self.time_controls.bullet,
                                            loading,
                                            |filter| &mut filter.bullet,
                                        ),
                                    )
                                    .child(
                                        self.render_time_control_checkbox(
                                            cx,
                                            "tc-blitz",
                                            "Blitz",
                                            self.time_controls.blitz,
                                            loading,
                                            |filter| &mut filter.blitz,
                                        ),
                                    )
                                    .child(
                                        self.render_time_control_checkbox(
                                            cx,
                                            "tc-rapid",
                                            "Rapid",
                                            self.time_controls.rapid,
                                            loading,
                                            |filter| &mut filter.rapid,
                                        ),
                                    )
                                    .child(
                                        self.render_time_control_checkbox(
                                            cx,
                                            "tc-classical",
                                            "Classical",
                                            self.time_controls.classical,
                                            loading,
                                            |filter| &mut filter.classical,
                                        ),
                                    ),
                            ),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Load")
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("load-games")
                                    .label(if loading {
                                        "Loading…"
                                    } else {
                                        "Load games"
                                    })
                                    .disabled(loading)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.load_games(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("stop-load")
                                    .label("Stop")
                                    .disabled(!loading)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.stop_loading(cx);
                                    })),
                            ),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Opening")
                    .child(
                        Label::new(opening_label)
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                GroupBox::new()
                    .title("Engine")
                    .child(
                        v_flex()
                            .gap_1()
                            .children(if engines.is_empty() {
                                vec![Label::new("Add an engine in the Engine tab")
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .into_any_element()]
                            } else {
                                engines
                                    .iter()
                                    .map(|engine| {
                                        self.render_engine_pick_card(
                                            &format!("opening-tree-engine-{}", engine.id),
                                            engine,
                                            selected_engine == engine.id,
                                            EnginePickTarget::OpeningTree(session_id),
                                            cx,
                                        )
                                        .into_any_element()
                                    })
                                    .collect()
                            }),
                    ),
            )
            .child(self.render_navigation_group(
                "go-back",
                "go-forward",
                "go-start",
                history_index == 0,
                next_move_count == 0,
                history_index == 0,
                |this, cx| {
                    if let Some(session) = this.active_opening_tree_mut() {
                        let session_id = session.id;
                        session.go_back(cx);
                        this.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    }
                    cx.notify();
                },
                |this, cx| {
                    if let Some(session) = this.active_opening_tree_mut() {
                        let session_id = session.id;
                        session.go_forward_popular(cx);
                        this.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    }
                    cx.notify();
                },
                |this, cx| {
                    if let Some(session) = this.active_opening_tree_mut() {
                        let session_id = session.id;
                        session.go_to_history(0, cx);
                        this.refresh_opening_tree_eval_if_engine_selected(session_id, cx);
                    }
                    cx.notify();
                },
                cx,
            ));

        self.render_expanded_sidebar(
            "novelty-sidebar",
            "Novelty",
            SIDEBAR_WIDTH,
            status,
            scroll_body,
            cx,
        )
    }
}
