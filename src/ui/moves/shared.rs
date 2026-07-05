use gpui::*;
use gpui_component::scroll::ScrollableElement;
use gpui_component::ActiveTheme;
use gpui_component::{h_flex, v_flex};

#[derive(Clone, Copy, Debug)]
pub enum MovesTableColumn {
    Move,
    Stat,
    Results,
    Last,
}

impl MovesTableColumn {
    pub const OPENING_TREE: &'static [Self] = &[Self::Move, Self::Stat, Self::Results, Self::Last];
    pub const EXPLORER: &'static [Self] = &[Self::Move, Self::Stat, Self::Results];

    fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Stat => "Games",
            Self::Results => "Results",
            Self::Last => "Last",
        }
    }

    fn width(self) -> Option<f32> {
        match self {
            Self::Move => Some(44.),
            Self::Stat => Some(56.),
            Self::Results => None,
            Self::Last => Some(32.),
        }
    }
}

pub fn moves_table_header(
    cx: &App,
    columns: &[MovesTableColumn],
    stat_label: impl Into<SharedString>,
) -> AnyElement {
    let stat_label = stat_label.into();
    h_flex()
        .flex_shrink_0()
        .gap_2()
        .px_2()
        .py_1()
        .border_b_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().muted.opacity(0.35))
        .children(columns.iter().map(|column| {
            let label: SharedString = if matches!(column, MovesTableColumn::Stat) {
                stat_label.clone()
            } else {
                column.label().into()
            };
            let cell = div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .child(label);
            match column.width() {
                Some(width) => cell.w(px(width)).into_any_element(),
                None => cell.flex_1().min_w(px(80.)).into_any_element(),
            }
        }))
        .into_any_element()
}

pub fn render_scrollable_moves_table(
    header: AnyElement,
    rows: Vec<AnyElement>,
    empty_message: Option<impl Into<SharedString>>,
    cx: &App,
) -> AnyElement {
    if rows.is_empty() {
        return v_flex()
            .size_full()
            .min_h_0()
            .child(header)
            .child(
                div()
                    .p_3()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        empty_message
                            .map(Into::into)
                            .unwrap_or_else(|| "No moves.".into()),
                    ),
            )
            .into_any_element();
    }

    v_flex()
        .size_full()
        .min_h_0()
        .child(
            v_flex()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(header)
                .children(rows),
        )
        .into_any_element()
}

pub fn results_bar(cx: &App, details: &crate::graph::PositionDetails) -> AnyElement {
    let total = details.total();
    let bar = h_flex()
        .h(px(14.))
        .flex_1()
        .min_w(px(60.))
        .rounded_sm()
        .overflow_hidden()
        .border_1()
        .border_color(cx.theme().border);

    if total == 0 {
        return bar.bg(cx.theme().muted).into_any_element();
    }

    bar.children([
        results_segment(details.white_pct(), rgb(0xf5f5f5).into(), rgb(0x333333).into()),
        results_segment(details.draw_pct(), rgb(0x9ca3af).into(), rgb(0xf9fafb).into()),
        results_segment(details.black_pct(), rgb(0x1f2937).into(), rgb(0xf9fafb).into()),
    ])
    .into_any_element()
}

pub fn results_segment(percent: f32, fill: Hsla, text_color: Hsla) -> AnyElement {
    if percent <= 0. {
        return div().into_any_element();
    }
    let label = if percent >= 10. {
        format!("{percent:.0}%")
    } else {
        String::new()
    };
    div()
        .flex()
        .items_center()
        .justify_center()
        .h_full()
        .flex_basis(relative(percent / 100.))
        .bg(fill)
        .text_xs()
        .text_color(text_color)
        .child(label)
        .into_any_element()
}
