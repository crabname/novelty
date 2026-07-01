//! Application tabs: home launcher, opening tree sessions, and mode stubs.

use gpui::*;
use gpui_component::IconName;

use crate::analysis_session::AnalysisSession;
use crate::session::ProfileSession;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKind {
    Play,
    GameAnalysis,
    Puzzle,
    Repertoire,
    OpeningTree,
    Database,
    Engine,
    Settings,
}

impl TabKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Play => "Play",
            Self::GameAnalysis => "Game Analysis",
            Self::Puzzle => "Puzzle",
            Self::Repertoire => "Repertoire",
            Self::OpeningTree => "Opening Tree",
            Self::Database => "Database",
            Self::Engine => "Engine",
            Self::Settings => "Settings",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Play => "Play against an engine or a friend",
            Self::GameAnalysis => "Analyze games move by move",
            Self::Puzzle => "Solve tactical puzzles",
            Self::Repertoire => "Build and practice your repertoire",
            Self::OpeningTree => "Explore openings from online games",
            Self::Database => "Load and browse game databases",
            Self::Engine => "Load chess engine binaries",
            Self::Settings => "App preferences and appearance",
        }
    }

    pub fn game_modes() -> &'static [TabKind] {
        &[
            Self::Play,
            Self::GameAnalysis,
            Self::Puzzle,
            Self::Repertoire,
            Self::OpeningTree,
        ]
    }

    pub fn tool_modes() -> &'static [TabKind] {
        &[Self::Database, Self::Engine, Self::Settings]
    }

    pub fn is_implemented(self) -> bool {
        matches!(self, Self::OpeningTree | Self::Engine | Self::GameAnalysis)
    }

    pub fn icon(self) -> IconName {
        match self {
            Self::Play => IconName::Play,
            Self::GameAnalysis => IconName::Inspector,
            Self::Puzzle => IconName::Bot,
            Self::Repertoire => IconName::BookOpen,
            Self::OpeningTree => IconName::Map,
            Self::Database => IconName::HardDrive,
            Self::Engine => IconName::Cpu,
            Self::Settings => IconName::Settings,
        }
    }

    pub fn tile_id(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::GameAnalysis => "analysis",
            Self::Puzzle => "puzzle",
            Self::Repertoire => "repertoire",
            Self::OpeningTree => "opening-tree",
            Self::Database => "database",
            Self::Engine => "engine",
            Self::Settings => "settings",
        }
    }
}

pub enum AppTab {
    Home { id: u64 },
    OpeningTree { id: u64, session: ProfileSession },
    GameAnalysis { id: u64, session: AnalysisSession },
    Engines { id: u64 },
    Stub { id: u64, kind: TabKind },
}

impl AppTab {
    pub fn id(&self) -> u64 {
        match self {
            Self::Home { id } => *id,
            Self::OpeningTree { id, .. } => *id,
            Self::GameAnalysis { id, .. } => *id,
            Self::Engines { id } => *id,
            Self::Stub { id, .. } => *id,
        }
    }

    pub fn label(&self) -> SharedString {
        match self {
            Self::Home { .. } => "Home".into(),
            Self::OpeningTree { session, .. } => session.label.clone(),
            Self::GameAnalysis { session, .. } => session.label.clone(),
            Self::Engines { .. } => TabKind::Engine.label().into(),
            Self::Stub { kind, .. } => kind.label().into(),
        }
    }

    pub fn opening_tree_mut(&mut self) -> Option<&mut ProfileSession> {
        match self {
            Self::OpeningTree { session, .. } => Some(session),
            _ => None,
        }
    }

    pub fn opening_tree(&self) -> Option<&ProfileSession> {
        match self {
            Self::OpeningTree { session, .. } => Some(session),
            _ => None,
        }
    }

    pub fn game_analysis(&self) -> Option<&AnalysisSession> {
        match self {
            Self::GameAnalysis { session, .. } => Some(session),
            _ => None,
        }
    }

    pub fn game_analysis_mut(&mut self) -> Option<&mut AnalysisSession> {
        match self {
            Self::GameAnalysis { session, .. } => Some(session),
            _ => None,
        }
    }
}
