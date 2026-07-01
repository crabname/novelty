use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::tab::{Tab, TabBar};
use gpui_component::*;

use crate::app::NoveltyApp;

impl NoveltyApp {
    pub(crate) fn render_board(
        &self,
        tab_index: usize,
        cx: &App,
        shapes_preview: usize,
    ) -> impl IntoElement {
        let board = self
            .opening_tree_at(tab_index)
            .map(|session| session.board.clone());

        div()
            .id(SharedString::from(format!("novelty-board-{tab_index}")))
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .when_some(board, |this, board| this.child(board))
            .child(
                div()
                    .absolute()
                    .bottom_2()
                    .right_2()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{shapes_preview} arrows · thicker = more common")),
            )
    }

    pub(crate) fn render_app_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_tab;
        TabBar::new("app-tabs")
            .flex_shrink_0()
            .px_2()
            .pt_2()
            .selected_index(active)
            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                this.select_tab(*ix, window, cx);
            }))
            .children(self.tabs.iter().enumerate().map(|(index, tab)| {
                let mut tab_view = Tab::new().label(tab.label());
                let can_close = index != 0 && self.tabs.len() > 1;
                if can_close {
                    tab_view = tab_view.suffix(
                        Button::new(SharedString::from(format!("close-tab-{index}")))
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.close_tab(index, cx);
                            })),
                    );
                }
                tab_view
            }))
            .suffix(
                Button::new("new-home-tab")
                    .icon(IconName::Plus)
                    .ghost()
                    .xsmall()
                    .tooltip("New tab")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.add_home_tab(cx);
                    })),
            )
    }
}
