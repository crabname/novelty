# Repertoire: Import Variations from Player Games

## Goal

Build and extend a repertoire from a **linked player profile** (Lichess or Chess.com
username). When loading games for that profile, import only lines where the player
played as the **repertoire color** (`[Orientation]`). From the current (or start)
position, extract continuations up to depth **N** and merge them into the PGN tree
without duplicating existing variations.

This complements [repertoire-building.md](./repertoire-building.md) (database +
engine suggestions) with **personal game history** as the source.

## User story

> I create a Black repertoire linked to my Lichess account `myuser`. On load,
> Novelty fetches my games **as Black only**, walks them from the starting
> position, and fills the tree with lines I actually played. At move 3 I can
> refresh from the same profile to add how I continued from here.

## Profile-linked repertoire

A repertoire file is tied to one online profile:

```pgn
[Event "Caro-Kann"]
[Orientation "black"]
[Site "https://lichess.org/@/myuser"]
[White "?"]
[Black "myuser"]
```

| Header | Purpose |
|--------|---------|
| `Orientation` | Repertoire side — **only color used for import** (`white` / `black`) |
| `Site` | `lichess.org` or `chess.com` + username (or dedicated `Player` tag) |
| `White` / `Black` | Username on the repertoire side; opponent as `?` |

**Load from profile** (sidebar, when creating or opening):

1. User enters **username** + **site** (or picks from `profile_history`).
2. **Color** is taken from repertoire `Orientation` — not chosen separately.
3. App streams games via `fetch::stream_games` with `color = repertoire_side`.
4. From **start position** (or current node on manual refresh), extract lines to
   depth N and merge into tree.
5. Save PGN + `sync_opening_headers`.

Manual **Import from games…** at a later position uses the **same linked profile**
and the **same color rule**; only the anchor FEN changes.

---

## Color matching (required)

Import **must** include only games where the profile username played as the
repertoire color. No games as the opposite color.

| Repertoire `[Orientation]` | Games included |
|--------------------------|----------------|
| `white` | Username was **White** in that game |
| `black` | Username was **Black** in that game |

This matches the existing fetch layer:

- **Lichess** — `export_user(username).color(white|black)` ([`fetch/lichess.rs`](../src/fetch/lichess.rs)).
- **Chess.com** — `color_matches(game, username, color)` ([`fetch/chesscom.rs`](../src/fetch/chesscom.rs)).

The import UI **does not** offer a separate color filter. If the user wants a
White repertoire, they set **As White** at creation (`Orientation`); profile
import always uses that value.

If `Orientation` and linked username disagree with a game (e.g. wrong header),
that game is already excluded by the API / `color_matches` — no extra UI.

---

## Recommended UI placement

### Primary: **Load from profile** (sidebar, New / linked repertoire)

When creating or editing a profile-linked repertoire:

```
┌─ From profile ───────────────────┐
│ Username  [ myuser        ]    │
│ Site      (•) Lichess  ( ) .com│
│ Side      As Black  ← from     │
│           Orientation (fixed)  │
│ Period    [ Last 3 months ▼ ]  │
│ Depth     [====●=====] 6 plies │
│ [ Load & build repertoire ]    │
└────────────────────────────────┘
```

This is the **main** import path: fetch + merge from **start position** on first
load, or merge from **current position** on repeat.

### Secondary: **Import from games…** (Variations group)

For refresh at the **current node** without re-entering profile:

```
┌─ Variations ─────────────────────┐
│ [ Add variation ]                │
│ [ Promote to mainline ]          │
│ [ Import from games… ]           │
│   Profile: myuser @ Lichess      │
│   (read-only if linked)          │
└──────────────────────────────────┘
```

**Why sidebar**

- Profile + variation actions live together.
- Notation column stays navigation-only (context menu later optional).

### Entry requirements

- Repertoire file exists, or user is on **Create** with name + **As White/Black**.
- Linked profile username non-empty for load/import.
- Button disabled: *Link a profile username first* / *Create repertoire first*.

---

## Import modal (refresh at current position)

Triggered by **Import from games…**. Title: **Import from player games**.

```
┌─ Import from player games ────────────────────────────────┐
│ Position: after 1. e4 c6 2. d4 d5  (Caro-Kann · …)        │
│ Side: Black (from Orientation — games as Black only)       │
├───────────────────────────────────────────────────────────┤
│ Profile    myuser @ Lichess        [ Change… ]             │
│ Period     [ Last 3 months ▼ ]                             │
│ Time       ☑ Bullet  ☑ Blitz  ☐ Rapid  ☐ Classical        │
│ Depth      [====●=====] 6 plies from current position      │
│ Max lines  [ 20 ▼ ]     Min games per line [ 1 ]          │
├───────────────────────────────────────────────────────────┤
│ Preview …                                                  │
│                        [ Cancel ]  [ Import selected ]     │
└───────────────────────────────────────────────────────────┘
```

### Field behaviour

| Field | Source | Notes |
|-------|--------|-------|
| Position | `tree.current().fen` + opening label | Read-only |
| Side | `[Orientation]` | **Fixed**; drives `PlayerColor` for fetch |
| Profile | PGN headers + `profile_history` | Username + site; set on **Load from profile** |
| Period / time | Same as Opening Tree | `LoadPeriod`, `TimeControlFilter` |
| Depth N | Slider 1–15, default 6 | Plies from anchor position |
| Max lines | 10 / 20 / 50 | Top distinct continuations by count |
| Min games | 1–5 | Drop noise |

**No separate color control** — always `player_color_from_headers(headers)`.

### Preview row status

| Badge | Meaning |
|-------|---------|
| **NEW** | Full line not in tree |
| **exists** | Sequence already reachable |
| **partial** | Shared prefix; append suffix only |

Checkboxes: **NEW** and **partial** on by default.

### Progress states

1. **Idle** → 2. **Loading games…** (as `{user}` **as White/Black**) →
3. **Building lines…** → 4. **Preview** → 5. **Importing…** → 6. **Done**.

---

## Data pipeline

```
repertoire_side = player_color_from_headers([Orientation])
profile = username + site from PGN / form
        │
        ▼
stream_games(profile, site, period, filters,
             color = repertoire_side)   // ONLY games on that color
        │
        ▼
For each game PGN (mainline):
  replay until simplified_fen == anchor_fen
  (anchor = start on profile load, current node on refresh)
        │
        ▼
Collect next N plies → group by SAN sequence → game_count
        │
        ▼
Sort, cap max_lines, diff vs MoveTree
        │
        ▼
merge_variations → sync_opening_headers → save_to_file
```

### Position matching

`graph::simplified_fen` for equality.

### Line extraction

Mainline only for MVP; plies `p+1 .. p+N` after position match.

### Lichess optimisation (later)

Player opening explorer API for position-filtered games (still respect color).

---

## Merge into repertoire tree

Same as before: merge from anchor node; match existing SAN; add siblings;
skip full duplicates; auto-save when file open.

```rust
struct ImportFromGamesRequest {
    username: String,
    site: Site,
    period: LoadPeriod,
    time_controls: TimeControlFilter,
    anchor_fen: String,           // start or current
    repertoire_side: PlayerColor, // from Orientation only
    depth: u8,
    max_lines: usize,
    min_games: u32,
}

// repertoire_side MUST be passed to stream_games as `color`.
```

---

## Overlap with Opening Tree tab

| | Opening Tree | Repertoire (profile load) |
|---|----------------|---------------------------|
| Color filter | User picks As White/Black | **From `[Orientation]`** |
| Profile | Sidebar username | **Linked in repertoire PGN** |
| Output | `OpeningGraph` | `MoveTree` + file |
| Anchor | Current board | Start or current node |

Reuse `stream_games`, `profile_history`, Lichess token from settings.

---

## Settings defaults

| Key | Default |
|-----|---------|
| `repertoire_import_depth` | 6 |
| `repertoire_import_max_lines` | 20 |
| `repertoire_import_min_games` | 1 |
| `repertoire_import_last_period` | Last 3 months |

---

## Implementation plan

### Phase 1 — Backend

1. PGN headers for linked profile (`Site` / player name).
2. `extract_continuations` with `color = repertoire_side` on fetch.
3. `merge_continuations` + tests.

### Phase 2 — UI

1. **Load from profile** on create/open (username, site, depth).
2. **Import from games…** at current node (profile read-only if linked).
3. Preview + merge + save.

### Phase 3 — Polish

1. Lichess player explorer fast path.
2. Notation context menu.
3. Cache games per `(site, user, period, color)`.

---

## Related docs

- [repertoire-building.md](./repertoire-building.md) — database + engine suggestions
- [databases.md](./databases.md) — local cross-database search
