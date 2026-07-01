use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::label::Label;
use gpui_component::*;

use crate::app::{NoveltyApp, UciConnectionStatus};
use crate::engines::{display_path, is_executable, LocalEngine};

#[derive(Clone, Copy, Debug)]
pub(crate) enum EnginePickTarget {
    GameAnalysis(u64),
    OpeningTree(u64),
}

impl NoveltyApp {
    pub(crate) fn render_engine_pick_card(
        &self,
        element_id: &str,
        engine: &LocalEngine,
        selected: bool,
        target: EnginePickTarget,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = engine.id.clone();
        let executable = is_executable(&engine.path);
        let uci = self.uci_state(&id);
        let status_label = match uci.status {
            UciConnectionStatus::Connected => "connected",
            UciConnectionStatus::Connecting => "connecting…",
            UciConnectionStatus::Analyzing => "analyzing…",
            UciConnectionStatus::Error => "error",
            UciConnectionStatus::Disconnected => "",
        };
        let engine_id = id.clone();

        div()
            .id(SharedString::from(element_id))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                cx.theme().accent
            } else {
                cx.theme().border
            })
            .bg(if selected {
                cx.theme().accent.opacity(0.08)
            } else {
                cx.theme().background
            })
            .when(!executable, |el| el.opacity(0.5))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if !executable {
                        return;
                    }
                    match target {
                        EnginePickTarget::GameAnalysis(tab_id) => {
                            this.select_analysis_engine(tab_id, &engine_id, cx);
                        }
                        EnginePickTarget::OpeningTree(session_id) => {
                            this.select_opening_tree_engine(session_id, &engine_id, cx);
                        }
                    }
                }),
            )
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        Label::new(engine.name.clone())
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM),
                    )
                    .child(
                        Label::new(display_path(&engine.path))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .truncate(),
                    )
                    .when(!executable, |col| {
                        col.child(
                            Label::new("Not executable")
                                .text_xs()
                                .text_color(cx.theme().warning),
                        )
                    })
                    .when(!status_label.is_empty(), |col| {
                        col.child(
                            Label::new(status_label)
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                        )
                    }),
            )
    }
}
