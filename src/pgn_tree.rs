//! PGN movetext with nested variations for repertoire trees.

use std::collections::HashMap;

use gpui_chessboard::Key;
use shakmaty::fen::Fen;
use shakmaty::{Chess, Move, Position, Square};

use crate::graph::{simplified_fen, start_fen};
use crate::move_tree::{MoveTree, TreeNode};

/// One piece of inline PGN notation — either plain text or a clickable move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotationSegment {
    pub text: String,
    pub position: Option<Vec<usize>>,
    pub is_variation: bool,
}

pub fn notation_segments(root: &TreeNode, current_position: &[usize]) -> Vec<NotationSegment> {
    let mut out = Vec::new();
    let mut move_number = 1;
    collect_sequence(
        root,
        &[],
        &mut out,
        fen_turn(&root.fen),
        &mut move_number,
        current_position,
    );
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MovePrefix {
    /// Mainline white: `1. e4`
    MainlineWhite,
    /// Mainline black replying on the same line: `e5`
    MainlineBlack,
    /// Variation white: `2. Nf3`
    VariationWhite,
    /// Variation black starting a branch: `1... c5`
    VariationBlack,
}

#[derive(Clone, Debug)]
pub struct ParsedRepertoire {
    pub label: String,
    pub headers: HashMap<String, String>,
    pub tree: MoveTree,
}

pub fn parse_repertoire_pgn(pgn: &str) -> Result<ParsedRepertoire, String> {
    let headers = crate::pgn::parse_headers(pgn);
    let movetext = crate::pgn::movetext_from_pgn(pgn).ok_or("PGN has no movetext")?;
    let start_fen = headers.get("FEN").cloned().unwrap_or_else(start_fen);
    let tree = parse_movetext_tree(&movetext, &start_fen)?;
    let label = headers
        .get("Event")
        .cloned()
        .unwrap_or_else(|| "Repertoire".to_string());
    Ok(ParsedRepertoire {
        label,
        headers,
        tree,
    })
}

pub fn format_repertoire_pgn(headers: &[(String, String)], tree: &MoveTree) -> String {
    let mut out = String::new();
    for (tag, value) in headers {
        out.push_str(&format!("[{tag} \"{value}\"]\n"));
    }
    out.push('\n');
    let movetext = tree_to_movetext(&tree.root);
    out.push_str(&movetext);
    if !movetext.contains('*') {
        out.push_str(" *");
    }
    out
}

pub fn format_repertoire_pgn_map(headers: &HashMap<String, String>, tree: &MoveTree) -> String {
    let pairs: Vec<(String, String)> = headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    format_repertoire_pgn(&pairs, tree)
}

fn parse_movetext_tree(movetext: &str, start_fen: &str) -> Result<MoveTree, String> {
    let tokens = tokenize_movetext(movetext);
    let mut parser = MovetextParser {
        tokens,
        index: 0,
        board: fen_to_chess(start_fen)?,
    };
    let mut root = TreeNode {
        fen: simplified_fen(&position_fen(&parser.board)),
        san: None,
        orig: None,
        dest: None,
        children: Vec::new(),
    };
    parser.parse_sequence(&mut root)?;
    Ok(MoveTree {
        root,
        position: Vec::new(),
        variation_mode: false,
    })
}

struct MovetextParser {
    tokens: Vec<MovetextToken>,
    index: usize,
    board: Chess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MovetextToken {
    San(String),
    VariationStart,
    VariationEnd,
}

impl MovetextParser {
    fn peek(&self) -> Option<&MovetextToken> {
        self.tokens.get(self.index)
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn parse_sequence(&mut self, attach_to: &mut TreeNode) -> Result<(), String> {
        let mut path = Vec::new();
        self.sync_board(attach_to, &path)?;
        loop {
            match self.peek() {
                None | Some(MovetextToken::VariationEnd) => break,
                Some(MovetextToken::VariationStart) => {
                    self.advance();
                    let mut parent_path = path.clone();
                    if !parent_path.is_empty() {
                        parent_path.pop();
                    }
                    self.parse_branch(attach_to, &parent_path)?;
                    if !matches!(self.peek(), Some(MovetextToken::VariationEnd)) {
                        return Err("Unclosed variation".into());
                    }
                    self.advance();
                    self.sync_board(attach_to, &path)?;
                }
                Some(MovetextToken::San(_)) => {
                    self.play_san_into(attach_to, &mut path)?;
                }
            }
        }
        Ok(())
    }

    fn parse_branch(&mut self, attach_to: &mut TreeNode, parent_path: &[usize]) -> Result<(), String> {
        let mut path = parent_path.to_vec();
        self.sync_board(attach_to, &path)?;
        loop {
            match self.peek() {
                Some(MovetextToken::VariationEnd) | None => break,
                Some(MovetextToken::VariationStart) => {
                    self.advance();
                    let mut alt_parent = path.clone();
                    if !alt_parent.is_empty() {
                        alt_parent.pop();
                    }
                    self.parse_branch(attach_to, &alt_parent)?;
                    if !matches!(self.peek(), Some(MovetextToken::VariationEnd)) {
                        return Err("Unclosed variation".into());
                    }
                    self.advance();
                    self.sync_board(attach_to, &path)?;
                }
                Some(MovetextToken::San(_)) => {
                    self.play_san_into(attach_to, &mut path)?;
                }
            }
        }
        Ok(())
    }

    fn sync_board(&mut self, attach_to: &TreeNode, path: &[usize]) -> Result<(), String> {
        let node = node_at_path(attach_to, path);
        self.board = fen_to_chess(&node.fen)?;
        Ok(())
    }

    fn play_san_into(&mut self, attach_to: &mut TreeNode, path: &mut Vec<usize>) -> Result<(), String> {
        let Some(san) = self.take_san() else {
            return Ok(());
        };
        let m: shakmaty::san::San = san
            .parse()
            .map_err(|_| format!("invalid SAN: {san}"))?;
        let mv = m
            .to_move(&self.board)
            .map_err(|_| format!("illegal move: {san}"))?;
        let (orig, dest) = move_keys(mv);
        self.board.play_unchecked(mv);
        let parent = node_at_mut_path(attach_to, path);
        parent.children.push(TreeNode {
            fen: simplified_fen(&position_fen(&self.board)),
            san: Some(san),
            orig: Some(orig),
            dest: Some(dest),
            children: Vec::new(),
        });
        let index = parent.children.len() - 1;
        path.push(index);
        Ok(())
    }

    fn take_san(&mut self) -> Option<String> {
        match self.peek() {
            Some(MovetextToken::San(san)) => {
                let san = san.clone();
                self.advance();
                Some(san)
            }
            _ => None,
        }
    }
}

fn tokenize_movetext(text: &str) -> Vec<MovetextToken> {
    let mut cleaned = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            while chars.next().is_some_and(|x| x != '}') {}
            continue;
        }
        if c == '(' || c == ')' {
            cleaned.push(' ');
            cleaned.push(c);
            cleaned.push(' ');
        } else {
            cleaned.push(c);
        }
    }

    let mut tokens = Vec::new();
    for raw in cleaned.split_whitespace() {
        match raw {
            "1-0" | "0-1" | "1/2-1/2" | "*" => break,
            "(" => tokens.push(MovetextToken::VariationStart),
            ")" => tokens.push(MovetextToken::VariationEnd),
            token if is_move_number(token) => {}
            token => {
                let san = strip_move_number_prefix(token);
                if !san.is_empty() {
                    tokens.push(MovetextToken::San(san));
                }
            }
        }
    }
    tokens
}

fn is_move_number(token: &str) -> bool {
    token.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn strip_move_number_prefix(token: &str) -> String {
    let mut rest = token;
    while let Some(stripped) = rest.strip_prefix(|c: char| c.is_ascii_digit()) {
        rest = stripped;
    }
    rest = rest.strip_prefix("...").unwrap_or(rest);
    rest = rest.strip_prefix('.').unwrap_or(rest);
    rest.to_string()
}

fn node_at_path<'a>(root: &'a TreeNode, path: &[usize]) -> &'a TreeNode {
    let mut node = root;
    for &index in path {
        node = &node.children[index];
    }
    node
}

fn node_at_mut_path<'a>(root: &'a mut TreeNode, path: &[usize]) -> &'a mut TreeNode {
    let mut node = root;
    for &index in path {
        node = &mut node.children[index];
    }
    node
}

fn tree_to_movetext(node: &TreeNode) -> String {
    let mut out = String::new();
    let mut move_number = 1;
    write_sequence(node, &mut out, fen_turn(&node.fen), &mut move_number);
    out.trim().to_string()
}

fn write_sequence(parent: &TreeNode, out: &mut String, white_to_move: bool, move_number: &mut usize) {
    let Some(main) = parent.children.first() else {
        return;
    };
    let branch_move_number = *move_number;
    let mut white_turn = white_to_move;
    let prefix = if white_turn {
        MovePrefix::MainlineWhite
    } else {
        MovePrefix::MainlineBlack
    };
    write_move_line(main, out, move_number, &mut white_turn, prefix);
    for variation in parent.children.iter().skip(1) {
        out.push_str(" (");
        let mut var_move_number = branch_move_number;
        write_variation_line(
            variation,
            out,
            fen_turn(&parent.fen),
            &mut var_move_number,
        );
        out.push(')');
    }
    write_sequence(main, out, white_turn, move_number);
}

fn write_variation_line(
    node: &TreeNode,
    out: &mut String,
    white_to_move: bool,
    move_number: &mut usize,
) {
    let mut white_turn = white_to_move;
    let prefix = if white_turn {
        MovePrefix::VariationWhite
    } else {
        MovePrefix::VariationBlack
    };
    write_move_line(node, out, move_number, &mut white_turn, prefix);
    if !node.children.is_empty() {
        out.push(' ');
        write_sequence(node, out, white_turn, move_number);
    }
}

fn write_move_line(
    node: &TreeNode,
    out: &mut String,
    move_number: &mut usize,
    white_turn: &mut bool,
    prefix: MovePrefix,
) {
    let san = node.san.as_deref().unwrap_or("--");
    match prefix {
        MovePrefix::MainlineWhite | MovePrefix::VariationWhite => {
            out.push_str(&format!("{move_number}. {san}"));
        }
        MovePrefix::VariationBlack => {
            out.push_str(&format!("{move_number}... {san}"));
        }
        MovePrefix::MainlineBlack => {
            out.push_str(san);
        }
    }
    if !node.children.is_empty() {
        out.push(' ');
    }
    if !*white_turn {
        *move_number += 1;
    }
    *white_turn = !*white_turn;
}

fn fen_turn(fen: &str) -> bool {
    fen.split_whitespace().nth(1).is_some_and(|t| t == "w")
}

fn fen_to_chess(fen_str: &str) -> Result<Chess, String> {
    let fen: Fen = fen_str
        .parse()
        .map_err(|_| format!("invalid FEN: {fen_str}"))?;
    fen.into_position(shakmaty::CastlingMode::Standard)
        .map_err(|_| format!("invalid position: {fen_str}"))
}

fn position_fen(pos: &Chess) -> String {
    Fen::from_position(pos, shakmaty::EnPassantMode::Legal).to_string()
}

fn move_keys(m: Move) -> (Key, Key) {
    let from = m.from().expect("chess move has origin");
    let to = m.to();
    (square_to_key(from), square_to_key(to))
}

fn square_to_key(sq: Square) -> Key {
    let file = sq.file().to_string();
    let rank = (sq.rank() as u8) + 1;
    Key::new(&format!("{file}{rank}")).expect("valid square")
}

fn push_text(out: &mut Vec<NotationSegment>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.position.is_none() {
            last.text.push_str(text);
            return;
        }
    }
    out.push(NotationSegment {
        text: text.to_string(),
        position: None,
        is_variation: false,
    });
}

fn push_move(
    out: &mut Vec<NotationSegment>,
    position: &[usize],
    san: &str,
    is_variation: bool,
) {
    out.push(NotationSegment {
        text: san.to_string(),
        position: Some(position.to_vec()),
        is_variation,
    });
}

fn collect_sequence(
    parent: &TreeNode,
    path: &[usize],
    out: &mut Vec<NotationSegment>,
    white_to_move: bool,
    move_number: &mut usize,
    current_position: &[usize],
) {
    let Some(main) = parent.children.first() else {
        return;
    };

    let branch_move_number = *move_number;
    let mut white_turn = white_to_move;
    let prefix = if white_turn {
        MovePrefix::MainlineWhite
    } else {
        MovePrefix::MainlineBlack
    };

    let main_path = {
        let mut p = path.to_vec();
        p.push(0);
        p
    };
    append_move_line(
        main,
        &main_path,
        out,
        move_number,
        &mut white_turn,
        false,
        prefix,
    );

    for (index, variation) in parent.children.iter().enumerate().skip(1) {
        push_text(out, " (");
        let var_path = {
            let mut p = path.to_vec();
            p.push(index);
            p
        };
        collect_variation_line(
            variation,
            &var_path,
            out,
            fen_turn(&parent.fen),
            branch_move_number,
            current_position,
        );
        push_text(out, ")");
    }

    collect_sequence(
        main,
        &main_path,
        out,
        white_turn,
        move_number,
        current_position,
    );
}

fn collect_variation_line(
    node: &TreeNode,
    path: &[usize],
    out: &mut Vec<NotationSegment>,
    white_to_move: bool,
    branch_move_number: usize,
    current_position: &[usize],
) {
    let mut move_number = branch_move_number;
    let mut white_turn = white_to_move;
    let prefix = if white_turn {
        MovePrefix::VariationWhite
    } else {
        MovePrefix::VariationBlack
    };
    append_move_line(node, path, out, &mut move_number, &mut white_turn, true, prefix);
    if !node.children.is_empty() {
        push_text(out, " ");
        collect_sequence(node, path, out, white_turn, &mut move_number, current_position);
    }
}

fn append_move_line(
    node: &TreeNode,
    path: &[usize],
    out: &mut Vec<NotationSegment>,
    move_number: &mut usize,
    white_turn: &mut bool,
    is_variation: bool,
    prefix: MovePrefix,
) {
    let san = node.san.as_deref().unwrap_or("--");
    match prefix {
        MovePrefix::MainlineWhite | MovePrefix::VariationWhite => {
            push_text(out, &format!("{move_number}. "));
            push_move(out, path, san, is_variation);
        }
        MovePrefix::VariationBlack => {
            push_text(out, &format!("{move_number}... "));
            push_move(out, path, san, is_variation);
        }
        MovePrefix::MainlineBlack => {
            push_move(out, path, san, is_variation);
        }
    }
    if !node.children.is_empty() {
        push_text(out, " ");
    }
    if !*white_turn {
        *move_number += 1;
    }
    *white_turn = !*white_turn;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_variation() {
        let pgn = "1. e4 e5 (1... c5) 2. Nf3 *";
        let tree = parse_movetext_tree(pgn, &start_fen()).unwrap();
        let e4 = &tree.root.children[0];
        assert_eq!(e4.san.as_deref(), Some("e4"));
        assert_eq!(e4.children.len(), 2);
        assert_eq!(e4.children[0].san.as_deref(), Some("e5"));
        assert_eq!(e4.children[1].san.as_deref(), Some("c5"));
    }

    #[test]
    fn round_trips_variation() {
        let pgn = "1. e4 e5 (1... c5 2. Nf3) 2. Nf3 Nc6 *";
        let tree = parse_movetext_tree(pgn, &start_fen()).unwrap();
        let formatted = format_repertoire_pgn(&[], &tree);
        let reparsed = parse_movetext_tree(&formatted, &start_fen()).unwrap();
        assert_eq!(
            reparsed.root.children[0].children.len(),
            tree.root.children[0].children.len()
        );
    }

    #[test]
    fn formats_mainline_move_numbers() {
        let pgn = "1. e4 e5 2. Nf3 Nc6 *";
        let tree = parse_movetext_tree(pgn, &start_fen()).unwrap();
        let movetext = tree_to_movetext(&tree.root);
        assert!(movetext.contains("1. e4 e5"), "{movetext}");
        assert!(movetext.contains("2. Nf3"), "{movetext}");
        let segments = notation_segments(&tree.root, &[]);
        let text: String = segments.iter().map(|s| s.text.as_str()).collect();
        assert!(text.contains("1. e4 e5"), "{text}");
        assert!(text.contains("2. Nf3"), "{text}");
        assert!(!text.contains("1. Nf3"), "{text}");
    }

    #[test]
    fn notation_segments_inline_variation() {
        let pgn = "1. e4 e5 (1... c5) 2. Nf3 *";
        let tree = parse_movetext_tree(pgn, &start_fen()).unwrap();
        let segments = notation_segments(&tree.root, &[0, 0]);
        let text: String = segments.iter().map(|s| s.text.as_str()).collect();
        assert!(text.contains("1. e4"), "{text}");
        assert!(text.contains("e5"), "{text}");
        assert!(text.contains("(1... c5)"), "{text}");
        assert!(text.contains("2. Nf3"), "{text}");
        assert!(segments.iter().any(|s| s.position.as_deref() == Some([0, 1].as_slice())));
    }
}
