//! Shared engine analysis state used by board sessions.

use gpui_chessboard::config::EvalConfigPatch;
use gpui_chessboard::{DrawShape, EvalBarPosition};

use crate::analysis_session::AnalysisSettings;
use crate::engine_shapes::engine_line_shapes;
use crate::engine_uci::AnalysisResult;

#[derive(Clone, Debug)]
pub struct EngineState {
    pub selected_engine_id: Option<String>,
    pub analyzing: bool,
    pub analysis: Option<AnalysisResult>,
    pub settings: AnalysisSettings,
}

impl EngineState {
    pub fn board_shapes(&self) -> Vec<DrawShape> {
        if self.settings.show_engine_lines
            && let Some(analysis) = &self.analysis
        {
            engine_line_shapes(analysis)
        } else {
            Vec::new()
        }
    }

    pub fn eval_patch(&self) -> Option<EvalConfigPatch> {
        if self.selected_engine_id.is_some() {
            Some(EvalConfigPatch {
                enabled: Some(true),
                position: Some(EvalBarPosition::Left),
                display: Some(if self.analyzing {
                    None
                } else {
                    self.analysis.as_ref().and_then(|result| result.best_eval())
                }),
            })
        } else {
            Some(EvalConfigPatch {
                enabled: Some(false),
                ..Default::default()
            })
        }
    }

    pub fn set_eval_pending(&mut self) {
        self.analyzing = true;
        self.analysis = None;
    }

    pub fn apply_analysis(&mut self, result: &AnalysisResult) {
        self.analyzing = false;
        self.analysis = Some(result.clone());
    }

    pub fn clear_analysis(&mut self) {
        self.analyzing = false;
        self.analysis = None;
    }
}
