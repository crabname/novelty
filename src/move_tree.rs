//! Variation tree for repertoire lines (mainline = child 0).

use gpui_chessboard::Key;

use crate::graph::{play_move_keys, start_fen};
use crate::session::HistoryStep;

#[derive(Clone, Debug, Default)]
pub struct TreeNode {
    pub fen: String,
    pub san: Option<String>,
    pub orig: Option<Key>,
    pub dest: Option<Key>,
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Debug)]
pub struct MoveTree {
    pub root: TreeNode,
    /// Path of child indices from root to the current position.
    pub position: Vec<usize>,
    /// Next board move is inserted as a variation (sibling) even if mainline is free.
    pub variation_mode: bool,
}

impl MoveTree {
    pub fn new() -> Self {
        Self::from_fen(start_fen())
    }

    pub fn from_fen(fen: String) -> Self {
        Self {
            root: TreeNode {
                fen,
                san: None,
                orig: None,
                dest: None,
                children: Vec::new(),
            },
            position: Vec::new(),
            variation_mode: false,
        }
    }

    pub fn current(&self) -> &TreeNode {
        self.node_at(&self.position).unwrap_or(&self.root)
    }

    pub fn current_mut(&mut self) -> &mut TreeNode {
        let position = self.position.clone();
        if position.is_empty() {
            return &mut self.root;
        }
        Self::node_at_mut(&mut self.root, &position).expect("invalid position")
    }

    pub fn node_at(&self, path: &[usize]) -> Option<&TreeNode> {
        Self::node_at_root(&self.root, path)
    }

    fn node_at_root<'a>(root: &'a TreeNode, path: &[usize]) -> Option<&'a TreeNode> {
        let mut node = root;
        for &index in path {
            node = node.children.get(index)?;
        }
        Some(node)
    }

    fn node_at_mut<'a>(root: &'a mut TreeNode, path: &[usize]) -> Option<&'a mut TreeNode> {
        let mut node = root;
        for &index in path {
            node = node.children.get_mut(index)?;
        }
        Some(node)
    }

    pub fn path_fens(&self) -> Vec<String> {
        let mut fens = vec![self.root.fen.clone()];
        let mut path = Vec::new();
        for &index in &self.position {
            path.push(index);
            if let Some(node) = self.node_at(&path) {
                fens.push(node.fen.clone());
            }
        }
        fens
    }

    pub fn mainline_steps(&self) -> Vec<&TreeNode> {
        let mut steps = vec![&self.root];
        let mut node = &self.root;
        while let Some(child) = node.children.first() {
            steps.push(child);
            node = child;
        }
        steps
    }

    pub fn make_move_from_board(
        &mut self,
        orig: &Key,
        dest: &Key,
    ) -> Result<(), String> {
        let fen = self.current().fen.clone();
        let (target_fen, san, orig_key, dest_key) = play_move_keys(&fen, orig, dest)?;
        self.make_step(target_fen, san, orig_key, dest_key);
        Ok(())
    }

    pub fn make_step(
        &mut self,
        fen: String,
        san: String,
        orig: Key,
        dest: Key,
    ) {
        let as_mainline = self.variation_mode;
        self.variation_mode = false;

        let parent_path = self.position.clone();

        let existing_index = {
            let parent = if parent_path.is_empty() {
                &self.root
            } else {
                Self::node_at_root(&self.root, &parent_path).unwrap_or(&self.root)
            };
            parent
                .children
                .iter()
                .position(|child| child.san.as_deref() == Some(san.as_str()))
        };
        if let Some(index) = existing_index {
            self.position.push(index);
            return;
        }

        let node = TreeNode {
            fen,
            san: Some(san),
            orig: Some(orig),
            dest: Some(dest),
            children: Vec::new(),
        };

        let child_index = if parent_path.is_empty() {
            if as_mainline {
                self.root.children.insert(0, node);
                0
            } else {
                self.root.children.push(node);
                self.root.children.len() - 1
            }
        } else {
            let parent = Self::node_at_mut(&mut self.root, &parent_path).expect("invalid position");
            if as_mainline {
                parent.children.insert(0, node);
                0
            } else {
                parent.children.push(node);
                parent.children.len() - 1
            }
        };
        self.position.push(child_index);
    }

    pub fn go_to_position(&mut self, position: Vec<usize>) {
        if self.node_at(&position).is_some() {
            self.position = position;
        }
    }

    pub fn go_back(&mut self) -> bool {
        if self.position.is_empty() {
            return false;
        }
        self.position.pop();
        true
    }

    pub fn go_forward_mainline(&mut self) -> bool {
        if self.current().children.is_empty() {
            return false;
        }
        self.position.push(0);
        true
    }

    pub fn next_branch(&mut self) -> bool {
        if self.position.is_empty() {
            return false;
        }
        let parent_path = self.position[..self.position.len() - 1].to_vec();
        let branch_index = *self.position.last().unwrap();
        let parent = match Self::node_at_root(&self.root, &parent_path) {
            Some(node) => node,
            None => return false,
        };
        if parent.children.len() <= 1 {
            return false;
        }
        let next = (branch_index + 1) % parent.children.len();
        *self.position.last_mut().unwrap() = next;
        true
    }

    pub fn previous_branch(&mut self) -> bool {
        if self.position.is_empty() {
            return false;
        }
        let parent_path = self.position[..self.position.len() - 1].to_vec();
        let branch_index = *self.position.last().unwrap();
        let parent = match Self::node_at_root(&self.root, &parent_path) {
            Some(node) => node,
            None => return false,
        };
        if parent.children.len() <= 1 {
            return false;
        }
        let prev = (branch_index + parent.children.len() - 1) % parent.children.len();
        *self.position.last_mut().unwrap() = prev;
        true
    }

    pub fn promote_current_variation(&mut self) -> bool {
        if self.position.is_empty() {
            return false;
        }
        let branch_index = *self.position.last().unwrap();
        if branch_index == 0 {
            return false;
        }
        let parent_path = self.position[..self.position.len() - 1].to_vec();
        if parent_path.is_empty() {
            let node = self.root.children.remove(branch_index);
            self.root.children.insert(0, node);
        } else {
            let parent = Self::node_at_mut(&mut self.root, &parent_path).expect("invalid position");
            let node = parent.children.remove(branch_index);
            parent.children.insert(0, node);
        }
        *self.position.last_mut().unwrap() = 0;
        true
    }

    pub fn branch_options(&self) -> Vec<(usize, String)> {
        if self.position.is_empty() {
            return self
                .root
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, child)| child.san.clone().map(|san| (i, san)))
                .collect();
        }
        let parent_path = &self.position[..self.position.len() - 1];
        let parent = Self::node_at_root(&self.root, parent_path).unwrap_or(&self.root);
        parent
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, child)| child.san.clone().map(|san| (i, san)))
            .collect()
    }

    pub fn current_branch_index(&self) -> Option<usize> {
        self.position.last().copied()
    }

    pub fn last_move_keys(&self) -> Option<(Key, Key)> {
        let node = self.current();
        match (&node.orig, &node.dest) {
            (Some(orig), Some(dest)) => Some((orig.clone(), dest.clone())),
            _ => None,
        }
    }

    pub fn to_history_steps(&self) -> Vec<HistoryStep> {
        self.mainline_steps()
            .iter()
            .map(|node| HistoryStep {
                fen: node.fen.clone(),
                san: node.san.clone(),
                orig: node.orig.clone(),
                dest: node.dest.clone(),
            })
            .collect()
    }
}
