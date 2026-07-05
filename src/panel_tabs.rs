//! Right-side panel tabs shared across board modes.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SidePanelTab {
    #[default]
    Moves,
    Explorer,
    Engine,
    Game,
}

impl SidePanelTab {
    pub const OPENING_TREE: &'static [SidePanelTab] = &[Self::Moves, Self::Engine];
    pub const GAME_ANALYSIS: &'static [SidePanelTab] = &[Self::Engine, Self::Explorer, Self::Game];
    pub const REPERTOIRE: &'static [SidePanelTab] = &[Self::Explorer];

    pub fn label(self) -> &'static str {
        match self {
            Self::Moves => "Moves",
            Self::Explorer => "Explorer",
            Self::Engine => "Engine",
            Self::Game => "Game",
        }
    }

    pub fn index_in(self, tabs: &[SidePanelTab]) -> usize {
        tabs.iter()
            .position(|tab| *tab == self)
            .unwrap_or(0)
    }

    pub fn from_index(tabs: &[SidePanelTab], index: usize) -> Self {
        tabs.get(index).copied().unwrap_or(tabs[0])
    }
}
