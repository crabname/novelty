# Game Databases: Storage and Cross-Database Search

## Context

In en-croissant, local position search works through a single **reference database**:
the user picks one base via a star icon or a board-side selector, and all queries
(`searchPosition`, repertoire coverage, novelty annotation) target one `.db3` file.

Novelty takes a different approach: **no default database selection** is required.
Search runs across all locally enabled databases at once.

## Decisions

### 1. Storage: one file per logical database

Each database is a standalone file:

```
{data_dir}/databases/
  {id}.db3          # SQLite games store
  {id}.ecsi         # mmap position-search index (optional)
```

**Why not a single monolithic file:**

- Simple incremental updates (re-download a player's games into their own file).
- Isolation on corruption or deletion.
- Pre-built databases can be downloaded and dropped in as separate files.
- No `VACUUM` needed when removing one database.

A monolith or vault container is only worth considering if hard requirements appear
for syncing a single file (e.g. cloud backup).

### 2. Search: cross-database, no reference DB

The user **does not pick** a primary database. When a position is opened in the
explorer, stats are aggregated across all **enabled** databases.

```
search_all(fen, filters, enabled_db_ids) -> PositionStats + sample games
```

MVP implementation:

1. Load the list of enabled databases from the catalog.
2. Call `search_position(db, fen, filters)` for each database in parallel.
3. **Merge** results:
   - move stats: sum `white` / `black` / `draw` per SAN;
   - sample games: top-N by rating from the combined pool.

### 3. Database catalog (lightweight SQLite or JSON)

A separate metadata catalog — not the games themselves:

```text
catalog:
  id            UUID
  path          path to .db3
  title         display name
  description   optional
  source        local | lichess | chesscom | download
  enabled       bool (included in cross-search, default: true)
  game_count    cached count
  updated_at    last update time
```

UI: per-database “include in search” checkboxes on the databases page — instead of
a reference-database star.

### 4. Caching

Cache key:

```text
(fen, filters_hash, sorted_enabled_db_ids)
```

Repeated queries for the same position and database set are served from cache.
When a database is updated, invalidate entries that include its `db_id`.

### 5. Data sources

| Source | Storage | Cross-search |
|--------|---------|--------------|
| Local PGN → convert | `.db3` | yes (if enabled) |
| Player games (Lichess) | `{user}_lichess.db3` | yes |
| Player games (Chess.com) | `{user}_chesscom.db3` | yes |
| Downloaded open database | `.db3` | yes |
| Lichess Opening Explorer (online) | not stored | separate mode / overlay |

Online explorer and local databases may share one UI, but they remain separate
sources technically (as in en-croissant: Local vs Lichess All).

## Position search algorithm (per database)

Inside each database, follow the en-croissant approach (no Zobrist in MVP):

1. Pre-filter: `pawn_home`, material.
2. Exact match: board comparison.
3. Partial match: bitboard containment.
4. Replay moves from the binary blob.

Cross-search does not change the per-database algorithm — it only adds an
aggregation layer on top.

## Deferred optimizations

Add only when needed, not in MVP:

| Optimization | When |
|--------------|------|
| Zobrist index (exact, ply ≤ 40) | search takes > 2–3 s on typical bases |
| Shared `.ecsi` with `database_id` | many databases; parallel per-db search too slow |
| LRU mmap: keep 2–3 indexes in memory | many databases, limited RAM |
| Monolithic vault file | mandatory single-file sync |

## Comparison with en-croissant

| | en-croissant | Novelty |
|---|---|---|
| Database for search | one reference DB, required | none; enabled flags |
| Cross-database search | no | yes, merge at API layer |
| Files | `{title}.db3` | `{uuid}.db3` + catalog |
| Startup preload | one reference DB | optional LRU or lazy load |

## API (draft)

```rust
/// Move stats after merging across all databases.
struct MergedMoveStats {
    san: String,
    white: u32,
    black: u32,
    draws: u32,
}

struct CrossDbSearchResult {
    moves: Vec<MergedMoveStats>,
    games: Vec<NormalizedGame>,  // top-N by rating
    total_positions: u32,        // sum of matches across databases
    searched_databases: Vec<DatabaseId>,
}

fn search_all(
    fen: &str,
    filters: SearchFilters,
    enabled: &[DatabaseId],
) -> CrossDbSearchResult;
```

## Implementation plan

1. **Catalog** — `catalog` with `enabled`, scan `*.db3`.
2. **`search_all`** — parallel per-db search + merge.
3. **Explorer UI** — no default DB selector; database filter panel (checkboxes).
4. **Cache** — keyed by `(fen, filters, enabled_set)`.
5. **Incremental updates** — unchanged: `convert_pgn(db_path, timestamp)`.

## Related plans

- [repertoire-from-games.md](./repertoire-from-games.md) — import continuations from
  a player's Lichess/Chess.com games at the current position into the repertoire tree.
- [repertoire-building.md](./repertoire-building.md) — when composing a repertoire,
  surface popular next moves from local databases and Lichess, and verify candidate
  lines with the engine.
