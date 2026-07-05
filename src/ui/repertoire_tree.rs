//! Recursive PGN-style variation tree (en-croissant layout).

use std::mem;

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::*;

use crate::app::NoveltyApp;
use crate::move_tree::TreeNode;
use crate::pgn_tree::{advance_move_state, fen_turn, move_prefix_label};

const TREE_INDENT: f32 = 20.;
const TREE_BASE_INDENT: f32 = 8.;

enum TreeRow {
    Inline(Vec<AnyElement>),
    VariationGroup {
        branch_path: Vec<usize>,
        variation_count: usize,
        blocks: Vec<AnyElement>,
        nest_depth: usize,
    },
}

struct SequenceState {
    rows: Vec<TreeRow>,
    inline: Vec<AnyElement>,
}

impl SequenceState {
    fn flush_inline(&mut self) {
        if !self.inline.is_empty() {
            self.rows.push(TreeRow::Inline(mem::take(&mut self.inline)));
        }
    }

    fn finish(&mut self) {
        self.flush_inline();
    }
}

impl NoveltyApp {
    pub(crate) fn repertoire_variation_tree_rows(
        &self,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let (root, current_position, tab_id) = self
            .repertoire_at(session_index)
            .map(|session| {
                (
                    session.tree.root.clone(),
                    session.tree.position.clone(),
                    session.id,
                )
            })
            .unwrap_or_else(|| (TreeNode::default(), Vec::new(), 0));

        let mut move_number = 1;
        let white_to_move = fen_turn(&root.fen);

        let mut state = SequenceState {
            rows: Vec::new(),
            inline: Vec::new(),
        };
        self.render_variation_sequence_into(
            cx,
            &mut state,
            &root,
            &[],
            &mut move_number,
            white_to_move,
            0,
            session_index,
            tab_id,
            &current_position,
        );
        state.finish();

        let mut elements = vec![div()
            .w_full()
            .flex_shrink_0()
            .child(self.repertoire_tree_start_cell(
                cx,
                session_index,
                tab_id,
                &current_position,
            ))
            .into_any_element()];

        elements.extend(
            state
                .rows
                .into_iter()
                .map(|row| self.render_tree_row_scroll_item(row, session_index, cx)),
        );
        elements
    }

    fn render_tree_row_scroll_item(
        &self,
        row: TreeRow,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .w_full()
            .flex_shrink_0()
            .child(self.render_tree_row(row, session_index, cx))
            .into_any_element()
    }

    fn render_tree_row(
        &self,
        row: TreeRow,
        session_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match row {
            TreeRow::Inline(cells) => h_flex()
                .w_full()
                .flex_wrap()
                .gap_x(px(2.))
                .gap_y(px(2.))
                .items_start()
                .content_start()
                .children(cells)
                .into_any_element(),
            TreeRow::VariationGroup {
                branch_path,
                variation_count,
                blocks,
                nest_depth,
            } => self.render_variation_group(
                cx,
                session_index,
                &branch_path,
                variation_count,
                nest_depth,
                blocks,
            ),
        }
    }

    fn render_variation_group(
        &self,
        cx: &mut Context<Self>,
        session_index: usize,
        branch_path: &[usize],
        variation_count: usize,
        nest_depth: usize,
        variation_blocks: Vec<AnyElement>,
    ) -> AnyElement {
        let key = crate::repertoire_session::variation_group_key(branch_path);
        let collapsed = self
            .repertoire_at(session_index)
            .is_some_and(|session| session.collapsed_variation_groups.contains(&key));
        let branch_path_owned = branch_path.to_vec();
        let toggle_id = SharedString::from(format!(
            "repertoire-var-toggle-{session_index}-{}",
            key
        ));

        v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_y(px(4.))
            .my_1()
            .pl(px(tree_indent_px(nest_depth)))
            .ml(px(TREE_BASE_INDENT))
            .border_l_2()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new(toggle_id)
                            .label(if collapsed { "▸" } else { "▾" })
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_repertoire_variation_group(
                                    session_index,
                                    branch_path_owned.clone(),
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if collapsed {
                                format!("{variation_count} variations")
                            } else {
                                String::new()
                            }),
                    ),
            )
            .when(!collapsed, |group| group.children(variation_blocks).gap_y(px(6.)))
            .into_any_element()
    }

    fn render_variation_sequence_into(
        &self,
        cx: &mut Context<Self>,
        state: &mut SequenceState,
        node: &TreeNode,
        path: &[usize],
        move_number: &mut usize,
        mut white_to_move: bool,
        variation_depth: usize,
        session_index: usize,
        tab_id: u64,
        current_position: &[usize],
    ) {
        let mut current = node;
        let mut path = path.to_vec();

        while let Some(main) = current.children.first() {
            let branch_move_number = *move_number;
            let white_at_parent = white_to_move;
            let main_path = extend_path(&path, 0);
            let main_path_for_click = main_path.clone();

            self.push_move_to_inline(
                cx,
                &mut state.inline,
                session_index,
                tab_id,
                &main_path,
                main.san.as_deref().unwrap_or("--"),
                false,
                *move_number,
                white_to_move,
                false,
                current_position,
                move |this, _, cx| {
                    if let Some(session) = this.repertoire_at_mut(session_index) {
                        session.go_to_position(main_path_for_click.clone(), cx);
                    }
                    this.refresh_explorer_if_needed(tab_id, cx);
                    cx.notify();
                },
            );

            (*move_number, white_to_move) = advance_move_state(white_to_move, *move_number);

            if current.children.len() > 1 {
                state.flush_inline();
                let nest_depth = variation_depth + 1;
                let variation_blocks: Vec<AnyElement> = current
                    .children
                    .iter()
                    .enumerate()
                    .skip(1)
                    .map(|(index, variation)| {
                        let var_path = extend_path(&path, index);
                        self.render_variation_branch(
                            cx,
                            variation,
                            &var_path,
                            branch_move_number,
                            white_at_parent,
                            nest_depth,
                            session_index,
                            tab_id,
                            current_position,
                        )
                    })
                    .collect();

                state.rows.push(TreeRow::VariationGroup {
                    branch_path: path.clone(),
                    variation_count: current.children.len() - 1,
                    blocks: variation_blocks,
                    nest_depth,
                });
            }

            path = main_path;
            current = main;
        }
    }

    fn push_move_to_inline(
        &self,
        cx: &mut Context<Self>,
        inline: &mut Vec<AnyElement>,
        session_index: usize,
        tab_id: u64,
        position: &[usize],
        san: &str,
        is_variation: bool,
        move_number: usize,
        white_to_move: bool,
        variation_entry: bool,
        current_position: &[usize],
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        inline.push(self.repertoire_tree_prefix(
            cx,
            move_prefix_label(move_number, white_to_move, variation_entry),
        ));
        inline.push(self.repertoire_tree_move_cell(
            cx,
            cell_id(session_index, position),
            san,
            position,
            is_variation,
            session_index,
            tab_id,
            current_position,
            on_click,
        ));
    }

    fn render_variation_branch(
        &self,
        cx: &mut Context<Self>,
        node: &TreeNode,
        path: &[usize],
        branch_move_number: usize,
        white_at_branch: bool,
        variation_depth: usize,
        session_index: usize,
        tab_id: u64,
        current_position: &[usize],
    ) -> AnyElement {
        let mut move_number = branch_move_number;
        let mut white_to_move = white_at_branch;
        let path_owned = path.to_vec();
        let path_for_click = path_owned.clone();

        let mut state = SequenceState {
            rows: Vec::new(),
            inline: Vec::new(),
        };

        self.push_move_to_inline(
            cx,
            &mut state.inline,
            session_index,
            tab_id,
            &path_owned,
            node.san.as_deref().unwrap_or("--"),
            true,
            move_number,
            white_to_move,
            true,
            current_position,
            move |this, _, cx| {
                if let Some(session) = this.repertoire_at_mut(session_index) {
                    session.go_to_position(path_for_click.clone(), cx);
                }
                this.refresh_explorer_if_needed(tab_id, cx);
                cx.notify();
            },
        );

        (move_number, white_to_move) = advance_move_state(white_to_move, move_number);

        self.render_variation_sequence_into(
            cx,
            &mut state,
            node,
            &path_owned,
            &mut move_number,
            white_to_move,
            variation_depth,
            session_index,
            tab_id,
            current_position,
        );
        state.finish();

        v_flex()
            .w_full()
            .gap_y(px(2.))
            .py_0p5()
            .children(
                state
                    .rows
                    .into_iter()
                    .map(|row| self.render_tree_row(row, session_index, cx)),
            )
            .into_any_element()
    }

    fn repertoire_tree_start_cell(
        &self,
        cx: &mut Context<Self>,
        session_index: usize,
        tab_id: u64,
        current_position: &[usize],
    ) -> AnyElement {
        self.repertoire_tree_move_cell(
            cx,
            SharedString::from(format!("repertoire-start-{session_index}")),
            "Start",
            &[],
            false,
            session_index,
            tab_id,
            current_position,
            move |this, _, cx| {
                if let Some(session) = this.repertoire_at_mut(session_index) {
                    session.go_to_position(Vec::new(), cx);
                }
                this.refresh_explorer_if_needed(tab_id, cx);
                cx.notify();
            },
        )
    }

    fn repertoire_tree_prefix(&self, cx: &Context<Self>, text: String) -> AnyElement {
        if text.is_empty() {
            return div().into_any_element();
        }
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(text)
            .into_any_element()
    }

    fn repertoire_tree_move_cell(
        &self,
        cx: &mut Context<Self>,
        id: SharedString,
        label: &str,
        position: &[usize],
        is_variation: bool,
        session_index: usize,
        _tab_id: u64,
        current_position: &[usize],
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let is_current = if label == "Start" {
            current_position.is_empty()
        } else {
            current_position == position
        };
        let is_on_line = if label == "Start" {
            !current_position.is_empty()
        } else {
            is_current || repertoire_position_on_line(position, current_position)
        };

        let menu_position = if label == "Start" {
            None
        } else {
            Some(position.to_vec())
        };

        let can_promote = menu_position.as_ref().is_some_and(|path| {
            self.repertoire_at(session_index)
                .is_some_and(|s| s.tree.can_promote_at(path))
        });
        let can_delete = menu_position.as_ref().is_some_and(|path| {
            self.repertoire_at(session_index)
                .is_some_and(|s| s.tree.can_delete_at(path))
        });
        let entity = cx.entity();

        let cell = div()
            .id(id)
            .px_1()
            .py_0p5()
            .rounded_sm()
            .cursor_pointer()
            .text_sm()
            .when(is_current, |el| {
                el.bg(cx.theme().accent.opacity(0.35))
                    .border_1()
                    .border_color(cx.theme().accent)
                    .font_weight(FontWeight::SEMIBOLD)
            })
            .when(!is_current && is_on_line, |el| el.bg(cx.theme().accent.opacity(0.12)))
            .when(!is_on_line, |el| el.hover(|s| s.bg(cx.theme().muted)))
            .when(is_variation && !is_current, |el| el.text_color(cx.theme().chart_2))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    on_click(this, window, cx);
                }),
            )
            .child(label.to_string());

        if let Some(path) = menu_position {
            cell.context_menu(move |menu, window, _cx| {
                let position_promote = path.clone();
                let position_delete = path.clone();
                menu.item(
                    PopupMenuItem::new("Promote to mainline")
                        .disabled(!can_promote)
                        .on_click(window.listener_for(&entity, move |this, _, window, cx| {
                            this.repertoire_tree_promote_at(
                                session_index,
                                position_promote.clone(),
                                window,
                                cx,
                            );
                        })),
                )
                .separator()
                .item(
                    PopupMenuItem::new("Delete line")
                        .disabled(!can_delete)
                        .on_click(window.listener_for(&entity, move |this, _, window, cx| {
                            this.repertoire_tree_delete_at(
                                session_index,
                                position_delete.clone(),
                                window,
                                cx,
                            );
                        })),
                )
            })
            .into_any_element()
        } else {
            cell.into_any_element()
        }
    }
}

fn extend_path(path: &[usize], index: usize) -> Vec<usize> {
    let mut next = path.to_vec();
    next.push(index);
    next
}

fn cell_id(session_index: usize, path: &[usize]) -> SharedString {
    SharedString::from(format!(
        "repertoire-pgn-{session_index}-{}",
        path.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("-")
    ))
}

pub(super) fn repertoire_position_on_line(path: &[usize], current: &[usize]) -> bool {
    !current.is_empty()
        && current.len() > path.len()
        && current.iter().zip(path.iter()).all(|(a, b)| a == b)
}

fn tree_indent_px(depth: usize) -> f32 {
    TREE_BASE_INDENT + TREE_INDENT * depth as f32
}
