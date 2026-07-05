# Repertoire Building: Database Suggestions and Engine Verification

## Goal

While composing a repertoire, Novelty should help the user discover and validate
candidate moves at each position:

1. **Database suggestions** — show popular next moves from game databases
   (local cross-database search and/or Lichess Opening Explorer).
2. **Engine verification** — check candidate lines against the active engine
   (eval, best move, principal variation).

The user keeps full control: suggestions are recommendations, not auto-inserted
moves unless explicitly accepted.

## Context

Today repertoire files are PGN trees on disk (`~/.config/novelty/repertoires/`).
The repertoire UI supports variations and navigation, but does not yet surface
database popularity or engine analysis when choosing what to add next.

en-croissant's **Build** tab in the practice panel is the closest reference:
it loads move stats from a single reference database via `searchPosition` and
computes tree coverage gaps. Novelty extends this with cross-database search
(see [databases.md](./databases.md)) and adds engine line checks.

## User flow

```
Current repertoire position (FEN)
        │
        ├─► Database layer: popular next moves (sorted by frequency)
        │       sources: enabled local DBs + optional Lichess explorer
        │
        ├─► Engine layer: top lines / eval for each candidate (and repertoire line)
        │
        └─► UI: combined panel — pick a move to add as a new variation
```

### Typical workflow

1. User navigates to a leaf or gap in the repertoire tree.
2. Panel shows **suggested moves** from databases (e.g. `1...c5 42%`, `1...e5 31%`).
3. For each suggestion (and for moves already in the repertoire), show **engine
   eval** and whether the line is consistent with engine top choices.
4. User clicks a suggestion → move is added as a variation (or main line).
5. Optionally continue along the engine PV for several plies to flesh out a line.

## Database suggestions

### Data sources

| Source | API | When to use |
|--------|-----|-------------|
| Local databases (cross-search) | `search_all(fen, filters, enabled_ids)` | offline, personal games, downloaded bases |
| Lichess Opening Explorer | existing `opening_explorer` / explorer API | large-sample popularity, online |

Both can be active simultaneously. UI merges move stats by SAN (sum counts).

### Filters (shared with explorer)

- Rating range (e.g. 2000+)
- Time control / speed (Lichess only)
- Date range
- Enabled local databases (catalog checkboxes)

### Ranking

Default sort: **total games** descending (popularity).

Optional secondary signals (later):

- win rate for the repertoire side;
- performance rating in user's own games (if player DBs are enabled).

### Minimum sample threshold

Ignore moves below `min_games` (configurable, default e.g. 10) to avoid noise
from single games — same idea as en-croissant's `coverageMinGamesAtom`.

### Caching

Reuse the cross-database cache from [databases.md](./databases.md):

```text
(fen, filters_hash, sorted_enabled_db_ids, source_mask)
```

`source_mask` distinguishes local-only vs Lichess-only vs combined.

## Engine verification

### Purpose

Popularity alone is insufficient: a move can be common but bad. Engine analysis
helps the user prefer sound repertoire choices and spot tactical holes in existing
lines.

### Per-position engine data

For the current FEN, request from the active UCI engine:

- **Eval** (centipawns or mate);
- **Best move** (and multi-PV top N, configurable default N=3);
- **Principal variation** (PV) for each PV line.

### Per-candidate checks

For each database-suggested SAN (and each move already in the repertoire):

| Check | Description |
|-------|-------------|
| In engine top-N? | Suggested move matches one of multi-PV lines |
| Eval after move | Engine eval of position after playing the move |
| Delta vs best | How much worse than engine's best move |
| PV alignment | How many plies of the user's line match engine PV |

### Line probing (deeper verification)

When user hovers or selects a candidate:

1. Play the move on the board.
2. Run engine for **K plies deep** along the repertoire continuation or engine PV
   (configurable depth / ply limit, e.g. 8–12 plies).
3. Flag lines where eval swings beyond a threshold (e.g. −0.5 cp for the
   repertoire side) — **gap** or **risk** indicator.

This mirrors "checking lines with the engine" without auto-rewriting the tree.

### Async / non-blocking

Engine and database fetches must not block the UI:

- Show loading state per panel section.
- Cancel stale requests when the user changes position.
- Debounce rapid navigation (same pattern as `opening_explorer`).

## UI (draft)

### Repertoire build panel sections

1. **Your moves** — moves already in the repertoire at this position.
2. **Popular moves** — database suggestions not yet in the repertoire.
3. **Engine** — best move, eval bar, multi-PV list (may overlap with analysis tab).

Each row:

```text
  SAN    database%    W/D/L bar    engine eval    [Add]
  c5     42%          ████░░       +0.3  (2nd)    [+]
```

Visual hints:

- ✓ green — in engine top-N and eval acceptable;
- ⚠ amber — popular but eval below threshold;
- ✗ red — engine disfavors (> threshold loss vs best).

### Actions

- **Add move** — insert as variation under current node.
- **Add engine PV** — add next K plies from a selected PV line.
- **Go to gap** — jump to next repertoire position with low database coverage
  (future: coverage map like en-croissant `computeTreeCoverage`).

## API (draft)

```rust
struct RepertoireSuggestion {
    san: String,
    white: u32,
    draws: u32,
    black: u32,
    source: SuggestionSource,  // Local | Lichess | Merged
    already_in_repertoire: bool,
}

struct EngineLineCheck {
    san: String,
    eval_cp: Option<i32>,
    mate_in: Option<i32>,
    pv: Vec<String>,
    rank: Option<u8>,       // 1..N in multi-PV, None if not in top-N
    delta_vs_best_cp: i32,
}

struct RepertoireBuildContext {
    fen: String,
    repertoire_side: Color,
    suggestions: Vec<RepertoireSuggestion>,
    engine_checks: Vec<EngineLineCheck>,
    engine_best: EngineLineCheck,
}

/// Fetch database suggestions (local + optional Lichess).
fn fetch_repertoire_suggestions(
    fen: &str,
    filters: SearchFilters,
    sources: DataSources,
) -> Vec<RepertoireSuggestion>;

/// Run engine multi-PV and compare against candidates.
fn verify_lines_with_engine(
    fen: &str,
    candidates: &[String],
    engine: &EngineHandle,
    multi_pv: u8,
) -> Vec<EngineLineCheck>;
```

## Coverage map (later phase)

Like en-croissant, walk the repertoire tree and for each node query database
move frequencies to compute **coverage** — how well the repertoire handles
opponent replies weighted by popularity.

```text
coverage(node) = Σ (freq(move) × coverage(child))  for covered moves
               + partial credit for high-frequency missing moves
```

Uses `search_all` at each node (expensive). Mitigations:

- cache per-node results;
- compute on demand for visible branch only;
- background job with progress indicator.

## Implementation plan

### Phase 1 — Database suggestions in repertoire UI

1. Wire repertoire panel to `search_all` for current FEN.
2. Optional toggle: include Lichess explorer (reuse `ExplorerHost` / fetch path).
3. Merge and display popular moves not yet in the tree.
4. **Add move** action on suggestion row.

### Phase 2 — Engine verification

1. On position change, run multi-PV for current FEN.
2. Annotate each suggestion with eval / rank / delta.
3. Threshold-based visual hints (acceptable vs risky).

### Phase 3 — Line probing and coverage

1. Deep PV preview on candidate selection.
2. Tree coverage map and "next gap" navigation.
3. Settings: `min_games`, eval threshold, multi-PV count, probe depth.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `repertoire_min_games` | 10 | Min database games to show a suggestion |
| `repertoire_engine_multi_pv` | 3 | Engine lines to compare |
| `repertoire_eval_threshold_cp` | 50 | Max loss vs best to still show as "OK" |
| `repertoire_probe_depth` | 10 | Plies to extend when probing a line |
| `repertoire_data_sources` | local + lichess | Which sources feed suggestions |

## Related docs

- [databases.md](./databases.md) — storage, cross-database search, catalog
