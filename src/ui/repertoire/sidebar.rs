use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::Button;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::fetch::PlayerColor;
use crate::repertoire::repertoire_display_name;

const REPERTOIRE_SIDEBAR_WIDTH: f32 = 300.;

impl NoveltyApp {
    pub(super) fn render_repertoire_sidebar(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if self.sidebar_collapsed {
            return self.render_collapsed_sidebar("repertoire-sidebar-collapsed", cx);
        }

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
        let profile_input = session.profile_input.clone();
        let import_site = session.import_site;
        let import_depth = session.import_depth;
        let import_loading = session.import_loading;
        let player_color = session.player_color();
        let saved_repertoires = self.list_repertoire_paths();

        let scroll_body = v_flex()
            .gap_3()
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
                                    Switch::new("repertoire-color-white")
                                        .label("As White")
                                        .checked(session.create_color == PlayerColor::White)
                                        .on_click(cx.listener(|this, checked, _, cx| {
                                            if let Some(session) = this.active_repertoire_mut() {
                                                session.create_color = if *checked {
                                                    PlayerColor::White
                                                } else {
                                                    PlayerColor::Black
                                                };
                                            }
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("repertoire-create")
                                        .label("Create")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.create_repertoire_from_ui(window, cx);
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
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.open_repertoire_path(path.clone(), window, cx);
                                    }))
                                    .into_any_element()
                                })),
                        ),
                )
            })
            .when(has_file, |panel| {
                panel.child(self.render_repertoire_import_section(
                    cx,
                    opening_label.clone(),
                    profile_input,
                    import_site,
                    import_depth,
                    import_loading,
                    player_color,
                ))
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
            .child(self.render_navigation_group(
                SharedString::from(format!("repertoire-back-{tab_id}")),
                SharedString::from(format!("repertoire-fwd-{tab_id}")),
                SharedString::from(format!("repertoire-start-{tab_id}")),
                at_start,
                !can_forward,
                at_start,
                move |this, cx| {
                    if let Some(session) = this.repertoire_by_id_mut(tab_id) {
                        session.go_back(cx);
                    }
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
                move |this, cx| {
                    if let Some(session) = this.repertoire_by_id_mut(tab_id) {
                        session.go_forward(cx);
                    }
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
                move |this, cx| {
                    if let Some(session) = this.repertoire_by_id_mut(tab_id) {
                        session.go_to_position(Vec::new(), cx);
                    }
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
                cx,
            ));

        self.render_expanded_sidebar(
            "repertoire-sidebar",
            "Repertoire",
            REPERTOIRE_SIDEBAR_WIDTH,
            status,
            scroll_body,
            cx,
        )
    }
}
