use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::scroll::ScrollableElement;
use gpui_component::*;

use crate::app::{NoveltyApp, UciConnectionStatus};
use crate::engine_catalog::{format_bytes, is_catalog_installed, CatalogOffer};
use crate::engines::{display_path, is_executable};

impl NoveltyApp {
    pub(crate) fn render_engines(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("novelty-engines")
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scrollbar()
            .p_6()
            .gap_4()
            .child(self.render_engines_header(cx))
            .child(self.render_catalog_section(cx))
            .child(self.render_local_file_section(cx))
            .child(self.render_loaded_engines_section(cx))
    }

    fn render_engines_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                Label::new("Engines")
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                Label::new(
                    "Download engines from GitHub Releases or add a local binary. \
                     Loaded engines are saved between restarts. Connect via UCI to analyze positions.",
                )
                .text_sm()
                .text_color(cx.theme().muted_foreground),
            )
            .when(!self.engine_status.is_empty(), |this| {
                this.child(
                    Label::new(self.engine_status.clone())
                        .text_sm()
                        .text_color(cx.theme().muted_foreground),
                )
            })
    }

    fn render_catalog_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        GroupBox::new()
            .title("Download from GitHub")
            .child(
                v_flex()
                    .gap_3()
                    .when(self.catalog_loading, |this| {
                        this.child(
                            Label::new("Loading catalog…")
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                    })
                    .when_some(self.catalog_error.clone(), |this, err| {
                        this.child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Label::new(err)
                                        .text_sm()
                                        .text_color(cx.theme().danger),
                                )
                                .child(
                                    Button::new("retry-engine-catalog")
                                        .label("Retry")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.refresh_engine_catalog(cx);
                                        })),
                                ),
                        )
                    })
                    .child({
                        let mut cards = Vec::new();
                        for offer in &self.catalog_offers {
                            cards.push(
                                self.render_catalog_card(offer, cx)
                                    .into_any_element(),
                            );
                        }
                        div().flex().flex_wrap().gap_4().children(cards)
                    }),
            )
    }

    fn render_catalog_card(
        &self,
        offer: &CatalogOffer,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let catalog_id = offer.engine.id.to_string();
        let installed = is_catalog_installed(&self.engines, offer.engine.id);
        let available = offer.available;
        let downloading = self
            .downloading_catalog_id
            .as_deref()
            .is_some_and(|id| id == offer.engine.id);
        let version_label = if offer.version.is_empty() {
            "Latest release".to_string()
        } else {
            offer.version.clone()
        };
        let size_label = offer
            .size
            .map(format_bytes)
            .unwrap_or_else(|| "—".to_string());

        v_flex()
            .id(SharedString::from(format!("catalog-card-{}", offer.engine.id)))
            .w(px(220.))
            .min_h(px(200.))
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().group_box)
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        Label::new(offer.engine.name)
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(
                        Label::new(offer.engine.description)
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        Label::new(format!("{version_label} · {size_label}"))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .when(!offer.available, |this| {
                        this.child(
                            Label::new("Not available on this platform")
                                .text_xs()
                                .text_color(cx.theme().warning),
                        )
                    }),
            )
            .child(
                Button::new(SharedString::from(format!(
                    "download-engine-{}",
                    offer.engine.id
                )))
                .label(if installed {
                    "Installed"
                } else if downloading {
                    "Downloading…"
                } else if available {
                    "Download"
                } else {
                    "Unavailable"
                })
                .disabled(!available || installed || downloading || self.catalog_loading)
                .w_full()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !installed && available {
                        this.download_catalog_engine(&catalog_id, cx);
                    }
                })),
            )
    }

    fn render_local_file_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        GroupBox::new()
            .title("Local file")
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        Label::new("Add one engine binary at a time from disk.")
                            .text_xs()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        Button::new("pick-engine-file")
                            .icon(IconName::FolderOpen)
                            .label("Choose engine file…")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pick_engine_file(cx);
                            })),
                    ),
            )
    }

    fn render_loaded_engines_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        GroupBox::new()
            .title("Loaded engines")
            .child(if self.engines.is_empty() {
                div()
                    .py_8()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Cpu)
                            .text_color(cx.theme().muted_foreground.opacity(0.5)),
                    )
                    .child(
                        Label::new("No engines loaded yet")
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .into_any_element()
            } else {
                v_flex()
                    .gap_2()
                    .children(self.engines.iter().map(|engine| {
                        self.render_engine_row(engine, cx).into_any_element()
                    }))
                    .into_any_element()
            })
    }

    fn render_engine_row(
        &self,
        engine: &crate::engines::LocalEngine,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let executable = is_executable(&engine.path);
        let id = engine.id.clone();
        let path_label = display_path(&engine.path);
        let uci = self.uci_state(&id);
        let path = std::path::PathBuf::from(&engine.path);
        let id_for_connect = id.clone();
        let id_for_disconnect = id.clone();
        let id_for_analyze = id.clone();

        v_flex()
            .id(SharedString::from(format!("engine-row-{}", engine.id)))
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().group_box)
            .child(
                h_flex()
                    .gap_3()
                    .items_start()
                    .child(
                        Icon::new(IconName::Cpu)
                            .text_color(cx.theme().foreground)
                            .mt(px(2.)),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .child(
                                Label::new(engine.name.clone())
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(
                                Label::new(path_label)
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .truncate(),
                            )
                            .when(!executable, |this| {
                                this.child(
                                    Label::new("File is not marked executable")
                                        .text_xs()
                                        .text_color(cx.theme().warning),
                                )
                            })
                            .when(!uci.identity.is_empty(), |this| {
                                this.child(
                                    Label::new(format!("UCI: {}", uci.identity))
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground),
                                )
                            })
                            .when(!uci.last_result.is_empty(), |this| {
                                this.child(
                                    Label::new(uci.last_result.clone())
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground),
                                )
                            }),
                    )
                    .child(
                        Button::new(SharedString::from(format!("remove-engine-{}", engine.id)))
                            .icon(IconName::Delete)
                            .ghost()
                            .tooltip("Remove")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_engine(&id, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .child(
                        Button::new(SharedString::from(format!("uci-connect-{}", engine.id)))
                            .label(match uci.status {
                                UciConnectionStatus::Connecting => "Connecting…",
                                UciConnectionStatus::Connected
                                | UciConnectionStatus::Analyzing => "Connected",
                                _ => "Connect UCI",
                            })
                            .disabled(
                                !executable
                                    || matches!(
                                        uci.status,
                                        UciConnectionStatus::Connecting
                                            | UciConnectionStatus::Connected
                                            | UciConnectionStatus::Analyzing
                                    ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.connect_uci_engine(id_for_connect.clone(), path.clone(), cx);
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("uci-analyze-{}", engine.id)))
                            .label(if uci.status == UciConnectionStatus::Analyzing {
                                "Analyzing…"
                            } else {
                                "Analyze startpos"
                            })
                            .disabled(
                                !matches!(
                                    uci.status,
                                    UciConnectionStatus::Connected | UciConnectionStatus::Analyzing
                                ),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.analyze_with_uci_engine(&id_for_analyze, cx);
                            })),
                    )
                    .when(
                        matches!(
                            uci.status,
                            UciConnectionStatus::Connected
                                | UciConnectionStatus::Analyzing
                                | UciConnectionStatus::Error
                        ),
                        |this| {
                            this.child(
                                Button::new(SharedString::from(format!(
                                    "uci-disconnect-{}",
                                    engine.id
                                )))
                                .label("Disconnect")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.disconnect_uci_engine(&id_for_disconnect, cx);
                                })),
                            )
                        },
                    ),
            )
    }
}
