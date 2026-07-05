use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::separator::Separator;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::pgn_tree::notation_segments;
use crate::repertoire::repertoire_display_name;

const REPERTOIRE_SIDEBAR_WIDTH: f32 = 300.;

impl NoveltyApp {
    pub(crate) fn render_repertoire(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if let Some(session) = self.active_repertoire_mut() {
            session.flush_pgn_ui_if_needed(window, cx);
        }

        let active = self.active_tab;

        h_flex()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .items_stretch()
            .child(self.render_repertoire_sidebar(window, cx))
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .items_stretch()
                    .child(self.render_repertoire_history(active, cx))
                    .child(
                        div()
                            .id(SharedString::from(format!("repertoire-board-{active}")))
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .overflow_hidden()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_handle.focus(window, cx);
                                }),
                            )
                            .child(self.render_repertoire_board(active)),
                    ),
            )
    }

    fn render_repertoire_sidebar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let session = self.active_repertoire().expect("repertoire tab");
        let tab_id = session.id;
        let status = session.status.clone();
        let at_start = session.tree.position.is_empty();
        let can_forward = !session.tree.current().children.is_empty();
        let branch_count = session.tree.branch_options().len();
        let has_file = session.file_path.is_some();
        let dirty = session.dirty;
        let opening_label = session.opening_label();
        let pgn_input = session.pgn_input.clone();
        let name_input = session.name_input.clone();
        let saved_repertoires = self.list_repertoire_paths();

        v_flex()
            .id("repertoire-sidebar")
            .flex_shrink_0()
            .w(px(REPERTOIRE_SIDEBAR_WIDTH))
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
                    .child(
                        Label::new("Repertoire")
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .gap_3()
                    .px_3()
                    .pb_3()
                    .overflow_y_scrollbar()
                    .when(!has_file, |panel| {
                        panel.child(
                            GroupBox::new()
                                .title("New")
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            Label::new("Name your repertoire")
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(Input::new(&name_input))
                                        .child(
                                            Button::new("repertoire-create")
                                                .label("Create")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.create_repertoire_from_ui(cx);
                                                })),
                                        ),
                                ),
                        )
                    })
                    .when(!saved_repertoires.is_empty(), |panel| {
                        panel.child(
                            GroupBox::new()
                                .title("Open")
                                .child(
                                    v_flex()
                                        .gap_1()
                                        .children(saved_repertoires.iter().map(|path| {
                                            let label = repertoire_display_name(path);
                                            let path = path.clone();
                                            Button::new(SharedString::from(format!(
                                                "open-repertoire-{}",
                                                path.display()
                                            )))
                                            .label(label)
                                            .w_full()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.open_repertoire_path(path.clone(), cx);
                                            }))
                                            .into_any_element()
                                        })),
                                ),
                        )
                    })
                    .child(
                        GroupBox::new()
                            .title("PGN")
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        Label::new("Synced with board moves")
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(Input::new(&pgn_input).h(px(160.)).w_full()),
                            ),
                    )
                    .child(
                        GroupBox::new()
                            .title("Variations")
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("repertoire-add-variation")
                                            .label("Add variation")
                                            .disabled(!has_file)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Some(session) = this.active_repertoire_mut() {
                                                    session.enable_variation_mode();
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("repertoire-promote")
                                            .label("Promote to mainline")
                                            .disabled(!has_file)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                if let Some(session) = this.active_repertoire_mut() {
                                                    session.promote_variation();
                                                    if session.file_path.is_some() {
                                                        let _ = session.save_to_file();
                                                    }
                                                }
                                                cx.notify();
                                            })),
                                    )
                                    .when(branch_count > 1, |group| {
                                        group.child(
                                            h_flex()
                                                .gap_1()
                                                .child(
                                                    Button::new("repertoire-prev-branch")
                                                        .label("◀ alt")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            if let Some(session) =
                                                                this.active_repertoire_mut()
                                                            {
                                                                session.previous_branch(cx);
                                                            }
                                                            cx.notify();
                                                        })),
                                                )
                                                .child(
                                                    Button::new("repertoire-next-branch")
                                                        .label("alt ▶")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            if let Some(session) =
                                                                this.active_repertoire_mut()
                                                            {
                                                                session.next_branch(cx);
                                                            }
                                                            cx.notify();
                                                        })),
                                                ),
                                        )
                                    }),
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
                            .title("File")
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("repertoire-save")
                                            .label(if dirty { "Save *" } else { "Save" })
                                            .disabled(!has_file)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.save_active_repertoire(cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        GroupBox::new()
                            .title("Navigation")
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "repertoire-back-{tab_id}"
                                        )))
                                        .label("←")
                                        .disabled(at_start)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(session) =
                                                this.repertoire_by_id_mut(tab_id)
                                            {
                                                session.go_back(cx);
                                            }
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "repertoire-fwd-{tab_id}"
                                        )))
                                        .label("→")
                                        .disabled(!can_forward)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(session) =
                                                this.repertoire_by_id_mut(tab_id)
                                            {
                                                session.go_forward(cx);
                                            }
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        Button::new(SharedString::from(format!(
                                            "repertoire-start-{tab_id}"
                                        )))
                                        .label("Start")
                                        .disabled(at_start)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if let Some(session) =
                                                this.repertoire_by_id_mut(tab_id)
                                            {
                                                session.go_to_position(Vec::new(), cx);
                                            }
                                            cx.notify();
                                        })),
                                    ),
                            ),
                    ),
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
    }

    fn render_repertoire_board(&self, tab_index: usize) -> impl IntoElement {
        let board = self
            .repertoire_at(tab_index)
            .map(|session| session.board.clone());

        div()
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .when_some(board, |this, board| this.child(board))
    }

    fn render_repertoire_history(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .flex_shrink_0()
            .w(px(220.))
            .h_full()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(self.render_repertoire_history_table(session_index, cx))
    }

    fn render_repertoire_history_table(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (segments, at_start) = self
            .repertoire_at(session_index)
            .map(|session| {
                (
                    notation_segments(&session.tree.root, &session.tree.position),
                    session.tree.position.is_empty(),
                )
            })
            .unwrap_or_default();

        let start_cell = div()
            .id(SharedString::from(format!("repertoire-start-{session_index}")))
            .px_1()
            .py_0p5()
            .rounded_sm()
            .cursor_pointer()
            .when(at_start, |el| el.bg(cx.theme().accent.opacity(0.25)))
            .when(!at_start, |el| el.hover(|s| s.bg(cx.theme().muted)))
            .text_sm()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if let Some(session) = this.repertoire_at_mut(session_index) {
                        session.go_to_position(Vec::new(), cx);
                    }
                    cx.notify();
                }),
            )
            .child("Start");

        let mut inline = vec![start_cell.into_any_element()];
        if !segments.is_empty() {
            inline.push(
                div()
                    .px_1()
                    .py_0p5()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("·")
                    .into_any_element(),
            );
        }

        for (index, segment) in segments.into_iter().enumerate() {
            if let Some(position) = segment.position {
                let selected = self.repertoire_at(session_index).is_some_and(|session| {
                    session.tree.position == position
                });
                inline.push(
                    div()
                        .id(SharedString::from(format!(
                            "repertoire-pgn-{}-{}",
                            session_index,
                            position
                                .iter()
                                .map(|i| i.to_string())
                                .collect::<Vec<_>>()
                                .join("-")
                        )))
                        .px_0p5()
                        .py_0p5()
                        .rounded_sm()
                        .cursor_pointer()
                        .when(selected, |el| el.bg(cx.theme().accent.opacity(0.25)))
                        .when(!selected, |el| el.hover(|s| s.bg(cx.theme().muted)))
                        .text_sm()
                        .when(segment.is_variation, |el| {
                            el.text_color(cx.theme().chart_2)
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _, cx| {
                                if let Some(session) = this.repertoire_at_mut(session_index) {
                                    session.go_to_position(position.clone(), cx);
                                }
                                cx.notify();
                            }),
                        )
                        .child(segment.text)
                        .into_any_element(),
                );
            } else {
                inline.push(
                    div()
                        .id(SharedString::from(format!(
                            "repertoire-pgn-text-{session_index}-{index}"
                        )))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(segment.text)
                        .into_any_element(),
                );
            }
        }

        v_flex()
            .size_full()
            .min_h_0()
            .px_2()
            .py_2()
            .overflow_y_scrollbar()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_x(px(2.))
                    .gap_y(px(2.))
                    .children(inline),
            )
    }
}
