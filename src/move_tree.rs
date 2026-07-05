//! Variation tree for repertoire lines (mainline = child 0).

use gpui_chessboard::Key;

use crate::graph::{play_move_keys, start_fen};
use crate::opening_book::{lookup_fens, OpeningMatch};

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

    pub fn mainline_fens(&self) -> Vec<String> {
        self.mainline_steps()
            .into_iter()
            .map(|node| node.fen.clone())
            .collect()
    }

    /// Deepest known opening on the mainline (child 0 at each step).
    pub fn mainline_opening(&self) -> Option<OpeningMatch> {
        let fens = self.mainline_fens();
        if fens.len() <= 1 {
            return None;
        }
        lookup_fens(&fens).filter(|opening| opening.eco != "—")
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
        self.promote_variation_at(&self.position.clone())
    }

    pub fn promote_variation_at(&mut self, position: &[usize]) -> bool {
        if position.is_empty() {
            return false;
        }
        let branch_index = *position.last().unwrap();
        if branch_index == 0 {
            return false;
        }
        let parent_path = position[..position.len() - 1].to_vec();
        if parent_path.is_empty() {
            let node = self.root.children.remove(branch_index);
            self.root.children.insert(0, node);
        } else {
            let parent = Self::node_at_mut(&mut self.root, &parent_path).expect("invalid position");
            let node = parent.children.remove(branch_index);
            parent.children.insert(0, node);
        }
        adjust_position_after_promote(&mut self.position, position);
        true
    }

    /// Remove the branch at `position` (this move and all continuations).
    pub fn delete_line_at(&mut self, position: &[usize]) -> bool {
        if position.is_empty() {
            return false;
        }
        let branch_index = *position.last().unwrap();
        let parent_path = &position[..position.len() - 1];
        let parent = if parent_path.is_empty() {
            &mut self.root
        } else {
            let Some(parent) = Self::node_at_mut(&mut self.root, parent_path) else {
                return false;
            };
            parent
        };
        if branch_index >= parent.children.len() {
            return false;
        }
        parent.children.remove(branch_index);
        adjust_position_after_delete(&mut self.position, position);
        true
    }

    pub fn can_promote_at(&self, position: &[usize]) -> bool {
        !position.is_empty() && position.last().copied().unwrap_or(0) != 0
    }

    pub fn can_delete_at(&self, position: &[usize]) -> bool {
        !position.is_empty()
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

    pub fn last_move_keys(&self) -> Option<(Key, Key)> {
        let node = self.current();
        match (&node.orig, &node.dest) {
            (Some(orig), Some(dest)) => Some((orig.clone(), dest.clone())),
            _ => None,
        }
    }

    /// Append `sans` as variations from `anchor_path`. Returns count of new plies created.
    pub fn merge_line_from_path(&mut self, anchor_path: &[usize], sans: &[String]) -> u32 {
        let mut path = anchor_path.to_vec();
        let mut added = 0u32;
        for san in sans {
            let Some(node) = Self::node_at_root(&self.root, &path) else {
                break;
            };
            if let Some(index) = node
                .children
                .iter()
                .position(|child| child.san.as_deref() == Some(san.as_str()))
            {
                path.push(index);
                continue;
            }
            let Ok((target_fen, _, orig, dest)) =
                crate::graph::play_san_at(&node.fen, san)
            else {
                break;
            };
            let child = TreeNode {
                fen: target_fen,
                san: Some(san.clone()),
                orig: Some(orig),
                dest: Some(dest),
                children: Vec::new(),
            };
            if path.is_empty() {
                self.root.children.push(child);
                path.push(self.root.children.len() - 1);
            } else {
                let Some(parent) = Self::node_at_mut(&mut self.root, &path) else {
                    break;
                };
                parent.children.push(child);
                path.push(parent.children.len() - 1);
            }
            added += 1;
        }
        added
    }
}

fn adjust_position_after_promote(current: &mut Vec<usize>, promoted: &[usize]) {
    if promoted.is_empty() {
        return;
    }
    if current.starts_with(promoted) {
        let mut new_pos = promoted[..promoted.len() - 1].to_vec();
        new_pos.push(0);
        new_pos.extend_from_slice(&current[promoted.len()..]);
        *current = new_pos;
        return;
    }
    let parent_len = promoted.len() - 1;
    let branch_index = promoted[parent_len];
    if current.len() > parent_len && current[..parent_len] == promoted[..parent_len] {
        let idx = current[parent_len];
        if idx == branch_index {
            current[parent_len] = 0;
        } else if idx > branch_index {
            current[parent_len] -= 1;
        }
    }
}

fn adjust_position_after_delete(current: &mut Vec<usize>, deleted: &[usize]) {
    if deleted.is_empty() {
        return;
    }
    if current.starts_with(deleted) {
        *current = deleted[..deleted.len() - 1].to_vec();
        return;
    }
    let parent_len = deleted.len() - 1;
    let branch_index = deleted[parent_len];
    if current.len() > parent_len && current[..parent_len] == deleted[..parent_len] {
        if current[parent_len] > branch_index {
            current[parent_len] -= 1;
        }
    }
}

#[cfg(test)]
mod line_edit_tests {
    use super::*;
    use gpui_chessboard::Key;

    fn step(tree: &mut MoveTree, _san: &str, orig: &str, dest: &str) {
        let (fen, san, o, d) = crate::graph::play_move_keys(
            tree.current().fen.as_str(),
            &Key::new(orig).unwrap(),
            &Key::new(dest).unwrap(),
        )
        .unwrap();
        tree.make_step(fen, san, o, d);
    }

    #[test]
    fn delete_line_removes_variation_and_moves_to_parent() {
        let mut tree = MoveTree::new();
        step(&mut tree, "e4", "e2", "e4");
        tree.go_to_position(Vec::new());
        step(&mut tree, "d4", "d2", "d4");
        tree.go_to_position(vec![0]);
        assert!(tree.delete_line_at(&[1]));
        assert_eq!(tree.root.children.len(), 1);
        assert_eq!(tree.position, vec![0]);
    }

    #[test]
    fn promote_variation_at_makes_branch_mainline() {
        let mut tree = MoveTree::new();
        step(&mut tree, "e4", "e2", "e4");
        tree.go_to_position(Vec::new());
        step(&mut tree, "d4", "d2", "d4");
        assert!(tree.promote_variation_at(&[1]));
        assert_eq!(tree.root.children[0].san.as_deref(), Some("d4"));
        assert_eq!(tree.position, vec![0]);
    }
}
