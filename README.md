# GH Projects

A custom GitHub dashboard — tiles that each run a `gh` CLI command to fetch live data, plus private notes tied to GitHub issues and PRs stored locally in SQLite.

**Single Rust app, no npm.** The frontend is built with [Dioxus](https://dioxuslabs.com) and compiles to WebAssembly; the Axum backend serves the compiled bundle and the REST API from one binary on one port.

## Features

- **Dashboards** — create multiple named dashboards, each with its own tile grid
- **Drag & resize tiles** — a 12-column grid (powered by [`hadrone-dioxus`](https://crates.io/crates/hadrone-dioxus)); drag a tile by its header, resize from the bottom-right handle. Position and size are persisted per tile to the database.
- **GH Query tiles** — each tile runs any `gh` CLI command and displays the JSON result as a sortable, filterable table
- **Note tiles** — embed a private note directly on a dashboard
- **Notes page** — full CRUD for private Markdown notes, optionally linked to a GitHub repo / issue / PR; one-click link to open the issue or PR on GitHub
- **War Rooms** — a free-style tracker for coordinated efforts (e.g. patching a CVE across several repos/releases). A war room is a list of **groups** (each bound to a repo), and each group holds **items** (a release/PR/issue) with a manual lifecycle **stage**, a free-text note, a checklist of steps, and an optional GitHub PR/issue link. Items inherit their group's repo. Repos come from a reusable, global **registry** (sidebar → *Repos*) so the same repos can be selected across war rooms.
- **Team Stats** — a PR authoring-vs-review dashboard across several repos. Answers "who opens PRs and who actually reviews them", tracks time-to-first-review and the unreviewed backlog, and lists the open PRs nobody has looked at. GitHub facts are cached locally in SQLite so each refresh only fetches what changed.
- **Calendar** — a release calendar that visualises planned and actual ship dates across multiple components on a monthly grid. Each calendar dashboard has colour-coded **components** (e.g. products or services) and **events** (a version string tied to a planned release date). Events carry a **status badge** (On track · At risk · Delayed · Released) and an optional **actual release date**; a countdown badge shows days remaining until the planned date. Event notes render as Markdown below the release row.

## Prerequisites

- Rust toolchain (`cargo`)
- The `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- The Dioxus CLI (`dx`): `cargo binstall dioxus-cli` (or `cargo install dioxus-cli`)
- `gh` CLI authenticated (`gh auth login`)
- [`cargo-make`](https://github.com/sagiegurari/cargo-make) — `cargo install cargo-make` — for the `cargo make start` convenience task (optional; the manual steps work without it)

## Running

### Quick start (`cargo make`)

With [`cargo-make`](https://github.com/sagiegurari/cargo-make) installed
(`cargo install cargo-make`), a single command builds the frontend bundle and
starts the backend:

```bash
cargo make start
```

This runs the `build-frontend` task (`dx build --release -p gh-projects-web`)
followed by `run-backend` (`cargo run -p gh-projects-backend`), then serves on
**http://localhost:3001**. (Tasks are defined in `Makefile.toml`.) The manual
two-step path below is equivalent if you'd rather not use `cargo-make`.

### 1. Build the frontend (WASM)

```bash
dx build --release -p gh-projects-web
```

This compiles the Dioxus app to `$CARGO_TARGET_DIR/dx/gh-projects-web/release/web/public/`
(`target/dx/...` unless you've set `CARGO_TARGET_DIR`).
(On some macOS setups `wasm-opt` aborts; `dx` falls back to an unoptimized but
fully working bundle — that's fine.)

> **`cargo build` does not rebuild the frontend.** The WASM bundle only changes
> when you run `dx build` (or `cargo make`, which does both). Restarting the
> backend after `cargo build` picks up backend changes only.

> **Rebuilding / changes not showing up?** `dx build` writes content-hashed
> assets into the output directory but doesn't prune old ones, so they pile up
> across builds. `index.html` itself isn't cache-busted, so after a rebuild your
> browser may still load the previous bundle — **hard-refresh** (Cmd+Shift+R) to
> pick up changes. To start from a clean bundle, delete the output directory
> first: `rm -rf "${CARGO_TARGET_DIR:-target}/dx/gh-projects-web/release/web/public"`.
>
> Still serving the old UI? The backend logs `Serving frontend bundle from <dir>`
> at startup — check that it matches where `dx build` wrote. A leftover
> `target/dx/...` bundle from before `CARGO_TARGET_DIR` was set is a classic
> cause; delete it or set `STATIC_DIR` explicitly.

### 2. Run the backend (serves the API **and** the frontend)

```bash
cargo run -p gh-projects-backend
```

The server starts on **http://localhost:3001** and serves the built frontend at
`/` plus the REST API under `/api`. Open <http://localhost:3001>. The SQLite
database file `gh-projects.db` is created automatically in the working
directory. If the frontend bundle hasn't been built yet, the backend still runs
the API and logs a warning; set `STATIC_DIR` to serve the bundle from a custom
location.

### Frontend dev mode (hot reload)

For iterating on the UI, run the backend (step 2) and, in another terminal:

```bash
dx serve -p gh-projects-web
```

This serves the app on its own port with hot reload and proxies `/api` to the
backend on :3001 (configured in `web/Dioxus.toml`).

Override defaults with environment variables:

```bash
DATABASE_URL=sqlite:///tmp/mydb.db PORT=8080 cargo run -p gh-projects-backend
```

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite://gh-projects.db` | SQLite connection string |
| `PORT` | `3001` | Port the backend listens on |
| `CACHE_DIR` | `.cache` | Directory where cached `gh` results are stored (relative to the working directory, or an absolute path) |
| `CACHE_TTL_SECS` | `300` | How long a cached result is considered fresh (seconds). After this, the next request re-runs `gh` and refreshes the file. |

Example — store the cache in `/tmp` and keep results for 10 minutes:

```bash
CACHE_DIR=/tmp/gh-cache CACHE_TTL_SECS=600 cargo run -p gh-projects-backend
```

| Variable | Default | Description |
|---|---|---|
| `STATIC_DIR` | `$CARGO_TARGET_DIR/dx/gh-projects-web/release/web/public` (`target/dx/...` if unset) | Directory of the built frontend bundle to serve |

## Adding a GH Query tile

1. Open a dashboard and click **Add tile**.
2. Select **GH Query** as the tile type.
3. Enter the `gh` command *without* the leading `gh ` prefix.

Examples:

| What you want | Command to paste |
|---|---|
| Open issues in a repo | `issue list --repo owner/repo --json number,title,state --limit 20` |
| Open PRs assigned to you | `pr list --assignee @me --json number,title,state,headRefName` |
| Recent releases | `release list --repo owner/repo --json tagName,name,publishedAt --limit 10` |
| Repo info | `repo view owner/repo --json name,description,stargazerCount,forkCount` |

The output columns are auto-detected from the JSON keys. You can optionally restrict which columns appear by editing the tile and adding a `columns` field (not yet exposed in the UI — edit the config JSON directly if needed).

## Field extractors

Some `gh` CLI fields return nested JSON objects (e.g. `author` is `{"login": "alice", ...}`). Field extractors tell the table which sub-field to display instead of the raw JSON blob.

### Built-in defaults

These are always applied with no configuration needed:

| Column | Extracted field |
|---|---|
| `author` | `login` |
| `repository` | `nameWithOwner` |
| `assignees` | `name\|login` |

### Global extractors (Settings → sidebar gear icon)

Open **Settings** from the bottom of the sidebar to add or override extractors that apply to every GH Query tile. The value is a plain JSON object:

```json
{
  "assignees": "login",
  "reviewRequests": "login"
}
```

Use `|` to specify fallback fields — the first non-empty value wins:

```json
{
  "assignees": "name|login"
}
```

Global extractors are merged on top of the built-in defaults, so you only need to specify what you want to change.

### Per-tile extractors (tile Edit dialog)

Open a tile's edit dialog (gear icon on hover) and add overrides in the **Field extractors override** box. These take precedence over both the built-in defaults and the global settings, and apply only to that tile:

```json
{
  "author": "name"
}
```

### Merge order

```
built-in defaults  →  global settings  →  per-tile override
```

Each layer only needs to declare what it changes. Sorting on a column always uses the extracted value, not the raw JSON.

## Backup & restore

**Settings → Export backup** writes a single JSON file covering dashboards
(with tiles, layout, row priority and per-tile column state), notes, war rooms,
calendars, release watches, the repo registry (including its Team Stats
`track_stats` flags), and the team roster. **Restore
backup** reads one back.

Restore **replaces** everything it manages — it wipes those tables first, then
recreates them from the file. Two consequences worth knowing:

- **Row ids change.** Dashboards, tiles, war-room items and so on are recreated,
  so their ids are new. Anything referencing them by id (`depends_on`,
  `source_item_id`, group `repo_id`, release-watch `repo_id`, per-tile
  localStorage keys) is re-pointed during the restore.
- **The repo registry is merged, not wiped.** Repos are matched on
  `owner_repo`; only genuinely new ones are added. Wiping them would
  cascade-delete release watches and null out war-room group links.

Before wiping anything, restore writes a copy of the current database next to
it as `gh-projects.db.pre-restore-<timestamp>` and reports the path. These are
never overwritten, so delete old ones when you don't need them.

A payload that would break a database constraint (unknown `tile_type`,
`ref_type`, or `member_group`) is rejected with a 400 **before** the wipe, so a
bad file leaves your data untouched. References that simply can't be resolved
are cleared rather than failing the restore, and the response reports how many:
`links_restored`, `links_dropped`, `release_watches_skipped`.

The Team Stats **fact tables are deliberately excluded** from backups. They are a
re-fetchable cache of public GitHub data, and restoring a sync cursor next to
zero facts would make every later sync fetch only the last few days, leaving the
dashboard silently empty. A restored database simply re-backfills. The per-repo
`track_stats` flag *is* configuration, so it does ride along. Restore never
touches the fact tables, so they survive a restore on the same machine.

Backup files are versioned (currently `5`). Older files still restore — fields
added since are optional, and a pre-`4` file without a `repos` section falls
back to treating `repo_id`s as local ids.

## Team & author groups

kgateway-style public repos mix your team's PRs with drive-by community
contributions. Open **Team** from the sidebar (just above Settings) to keep a
roster of GitHub logins, and every GH Query tile whose rows include `author`
gains an `authorGroup` column:

| Value | Meaning |
|---|---|
| `team` | login is on the roster as **Team** |
| `pe` | login is on the roster as **PE** — the Product Excellence team |
| `solo` | login is on the roster as **Solo** — a colleague who isn't on the team, PE, or a maintainer |
| `maintainer` | login is on the roster as **Maintainer** |
| `bot` | login is on the roster as **Bot**, or `gh` reports `author.is_bot`, or the login ends in `[bot]` |
| `community` | anyone else — `community` is the fallback, so it is never stored |

The column is a normal column: sort it, hide it or drag it in the **Columns**
picker, filter it from the funnel, or click a badge to filter the tile to that
group.

### Populating the roster

Add logins by hand, or add an **import source** and hit **Refresh from GitHub**:

- `owner/team-slug` — pulls `gh api orgs/{owner}/teams/{slug}/members`
- `owner` — pulls `gh api orgs/{owner}/members`

Refresh only *adds* logins that aren't on the roster yet — it never edits or
removes an existing row — so hand-curated groups like **Team**, **PE** and
**Solo** survive every refresh untouched, even when the same login also appears
in an imported team. A source that fails (bad slug, no access) is reported without
stopping the others.

Each login belongs to exactly one group, matched case-insensitively. The roster
and its import sources are included in Backup/Restore.

## Team Stats

AI-assisted authoring made opening a PR nearly free; reviewing stayed expensive.
**Team Stats** (sidebar → *Team Stats*) makes the resulting imbalance visible.

Five sections, ordered diagnosis → explanation → action:

1. **KPI strip** — unreviewed PRs older than 7 days, p90 time-to-first-review,
   merges nobody approved, review concentration, and AI-review volume.
2. **Review debt** — per person, with a *What do these columns mean?* toggle in
   the section itself:

   | Column | Meaning |
   |---|---|
   | **Opened** | PRs they authored, *created* inside the window, **that are asking for review**. Still-draft PRs are excluded — they request nothing, and counting them would penalise anyone who works in drafts |
   | **Drafts** | How many of their PRs are still drafts. Context only; never counted in Opened, so it moves neither ratio |
   | **Merged** | How many of those have since merged — counted by when the PR was *opened*, not when it merged |
   | **Merged / Opened** | Share of the PRs they opened that have merged **so far**. Merged counts by when a PR was *opened*, so one opened late in the window may not have had time to land — a low rate at the recent end is expected |
   | **Reviewed** | Distinct PRs *by someone else* they reviewed during the window; each PR counted once. Self-reviews and bots excluded |
   | **Approvals** | Times they hit *Approve* on someone else's PR in the window. Counts approval **events** where Reviewed counts distinct PRs, so they differ in both directions — re-approving after changes adds here only; commenting without approving adds to Reviewed only |
   | **Received** | Human reviews landed on the PRs they opened, over those PRs' whole lifetime — the review capacity they consumed |
   | **Queue** | Open non-draft PRs where they're a requested reviewer and haven't reviewed yet. A live snapshot, not window-scoped |
   | **Reviewed / Opened** | The reciprocity number. Below 1.00 they ask for more review than they give. `∞` when they opened nothing — undefined, not a score |
   | **Trend** | Distinct PRs reviewed per week; the same vertical scale on every row |

   Everything except Queue is scoped to the window and to the tracked repos.
   A member with no activity at all shows `—`; `∞` is reserved for someone who
   reviewed but opened nothing.
3. **Weekly trend** — opened vs. reviewed vs. AI reviews over time.
4. **PR health by repo** — open backlog, median and p90 time-to-first-review,
   merges with no approval, and the share that never got a human review.
5. **Needs a reviewer** — open PRs with no human review, showing who (if anyone)
   is on the hook. Filter to *AI only* (a bot reviewed it, no human did) or all
   open PRs, and flip between **oldest first** (what's rotting) and **newest
   first** (what just landed in the queue). Click a row to open the usual
   detail panel.

### The reporting window

The selector offers 1, 4, 8, 12 and 26 weeks, and each means that many
**complete** Monday-start weeks. The current, still-running week is always
excluded: a partial week drags every rate and average down for no reason other
than the clock, and on a Monday morning it would be empty. So every window ends
on the most recent Sunday, and longer windows simply reach further back from the
same end date.

Two things deliberately ignore the window, because they are about right now
rather than about a period: the **Queue** column, and the *Needs a reviewer*
list with its open-backlog KPIs.

### Tracked repos

Stats only cover repos you opt in: sidebar → *Repos* → tick **Stats**. Leave very
busy upstream repos off — they dominate the numbers without telling you much
about your team.

### Syncing

There is no background job. Hit **Sync now** when you want fresh numbers.

The first sync for a repo backfills from **2026-08-01**; later syncs fetch only
PRs updated since the last cursor, plus a full sweep of open PRs so the
backlog list is always exact (that sweep is why PRs older than the backfill
floor still show up in *Needs a reviewer*). For five repos that is roughly 450
PRs and 30 GraphQL points on a cold run, and ~15 points afterwards.

One repo failing never aborts the others — its error is reported and its cursor
left untouched, so the next run re-covers the same window.

### What counts as a review

Three exclusions do most of the work, and getting them wrong would invert the
signal this page exists to show:

- **Self-reviews don't count.** Authors commenting on their own PRs is common.
- **Bot reviews don't count** as human reviews — they get their own column.
  `copilot-pull-request-reviewer` is seeded into the roster as a **bot** because
  it carries no `[bot]` login suffix and, measured across five repos, submitted
  more reviews than any human.
- **Dismissed reviews** still count as a review *given* (real human effort) but
  not as an approval.

The bot/self rules are evaluated live against the **Team** roster, so
reclassifying a login takes effect on all historical numbers immediately —
no re-sync needed.

A few honest caveats the numbers can't fix:

- **Time-to-first-review is measured only over PRs that got one**, so it is
  always shown next to the share that never did. A median over the reviewed
  subset looks healthiest exactly when the backlog is worst.
- **Draft history is invisible** — only the *current* draft flag is available.
  So "still a draft" is accurate, but a PR that sat in draft for a week and was
  then marked ready counts as a normal PR, and its time-to-first-review is
  measured from when it was opened rather than from when it became reviewable.
- Week buckets are **Monday-start UTC**.

## How notes work

Notes are **private** — they live only in your local SQLite database and are never sent to GitHub.

Each note has:
- **Title** — short label shown in the sidebar
- **Body** — full Markdown content
- **Repo** — optional `owner/repo` association (e.g. `torvalds/linux`)
- **Ref type** — `issue`, `pr`, or `repo`
- **Issue/PR number** — when set alongside a repo + ref type, an external link icon appears that opens the GitHub page directly

You can embed a note in a dashboard as a **Note tile** — useful for pinning context, checklists, or investigation notes next to the relevant GH Query tiles.

## War Rooms

A **War Room** tracks a coordinated effort across several repos/releases. It's a
list of **groups** (each bound to a repo from the global registry), and each
group holds **items** (a release/PR/issue) with a manual lifecycle **stage**, a
note, a checklist, and an optional GitHub link. Extra capabilities:

- **Linked items** — mirror an item from another group; the mirror is read-only
  and reflects the source's live status (edit the original once, see it
  everywhere). Deleting the source removes its mirrors.
- **Copy / paste** — drop an independent copy of an item into another group.
- **Group rollup status** — each group header shows an overall status derived
  from its items (blocked wins; all-complete shows the furthest stage; any
  unstarted/in-flight work reads as *In progress*).
- **Generate Slack update** — see below.

### Slack status update

The **Slack update** button in a war room generates a ready-to-paste status
message. Groups that share a product prefix are nested under one header (e.g.
`kgateway OSS v2.3.3` and `kgateway OSS v2.2.6` group under **kgateway OSS**,
with `v2.3.3` / `v2.2.6` as release bullets and the items beneath each). Every
release bullet's emoji is its group rollup status; linked items show their
source group (`Envoy Releases / v1.37.4`). The dialog is editable before you
copy, and an **Include item details** toggle drops the per-release item list for
a headline-only view.

### Status emojis (Settings → sidebar gear icon)

The emoji used for each stage in the Slack update is configurable. Open
**Settings** and edit the **Status emojis** section — one input per stage. Use
your Slack workspace's emoji codes (e.g. `:white_check_mark:`). Defaults:

| Stage | Default emoji |
|---|---|
| To do | `:white_circle:` |
| In progress | `:waiting:` |
| Blocked | `:red_circle:` |
| Merged | `:large_blue_circle:` |
| Released | `:white_check_mark:` |
| Done | `:white_check_mark:` |

This is a **global** setting (stored in the browser's local storage, alongside
the field extractors), so it applies to every war room. Anything you don't
override falls back to the default above.

## Calendar

A **Calendar dashboard** tracks planned and actual release dates for one or more
components on a monthly grid. Create as many calendars as you like from the
sidebar.

### Components

A **component** represents a product, service, or team whose releases you want
to track (e.g. `Gateway`, `Control Plane`). Each component gets a colour that
tints its pills on the calendar grid. An optional **short name** is shown on the
calendar pill instead of the full name when space is tight.

### Events

An **event** is a single release entry with:

| Field | Description |
|---|---|
| **Component** | Which component owns this release (optional) |
| **Version** | Free-text version string, e.g. `v2.4.0` |
| **Planned release date** | The target ship date |
| **Actual release date** | Set this once the release ships; the pill on the calendar gets an outline to distinguish it |
| **Status** | See below |
| **Note** | Free-text Markdown; rendered below the event row in the sidebar list |

### Status badges

Each event has one of four statuses shown as a colour-coded badge on its row:

| Status | Colour | Meaning |
|---|---|---|
| On track | Green | Release is on schedule |
| At risk | Yellow | Some risk to the date |
| Delayed | Red | Release will slip |
| Released | Blue | Shipped; auto-assigned when the planned date is today or in the past |

When you enter or change the planned date, the status flips automatically
between *On track* and *Released* (skipping *At risk* / *Delayed* so manual
risk flags are preserved).

### Countdown badges

Events without an actual release date show a countdown badge next to the date:

- **today** — releases scheduled for today
- **in Nd / Nw** — days or weeks until the planned date
- **Nd ago** — overdue (past planned date, not yet released)

### Calendar navigation

The toolbar shows the current month with **‹** / **›** arrows to step forward
and back. A **Today** button appears whenever you've navigated away from the
current month and returns you to it instantly.

## Project structure

```
gh-projects/
├── Cargo.toml              # workspace (members: backend, web)
├── backend/                # Rust + Axum + sqlx; also serves the WASM bundle
│   ├── migrations/         # SQL migrations (run automatically on startup)
│   └── src/
│       ├── main.rs
│       ├── handlers/       # HTTP route handlers
│       ├── models/         # Serde structs
│       ├── repositories/   # DB access (trait + SQLite impl)
│       └── state.rs        # Shared app state
└── web/                    # Rust + Dioxus (compiles to WebAssembly)
    ├── Dioxus.toml         # app config + dev proxy
    ├── assets/main.css     # plain CSS (no Tailwind/npm)
    └── src/
        ├── api.rs          # async REST client (gloo-net)
        ├── types.rs        # serde types mirroring the API
        ├── state.rs        # shared signals (cache versions + settings)
        ├── components/     # Sidebar, tiles, dialogs, detail panel
        └── pages/          # Dashboard, Notes, Home
```

## API overview

| Method | Path | Description |
|---|---|---|
| GET/POST | `/api/notes` | List / create notes |
| GET/PUT/DELETE | `/api/notes/:id` | Get / update / delete note |
| GET/POST | `/api/dashboards` | List / create dashboards |
| GET/PUT/DELETE | `/api/dashboards/:id` | Get / update / delete dashboard |
| GET/POST | `/api/dashboards/:id/tiles` | List / create tiles |
| PUT/DELETE | `/api/dashboards/:id/tiles/:tid` | Update / delete tile |
| GET/POST | `/api/repos` | List / create repo-registry entries |
| PUT/DELETE | `/api/repos/:id` | Update / delete a repo-registry entry |
| GET/POST | `/api/team-members` | List / create team-roster entries |
| PUT/DELETE | `/api/team-members/:id` | Update / delete a roster entry |
| POST | `/api/team-members/refresh` | Re-import every source; returns `{added, skipped, errors}` |
| GET/POST | `/api/team-member-sources` | List / create roster import sources |
| DELETE | `/api/team-member-sources/:id` | Delete an import source |
| GET/POST | `/api/war-rooms` | List / create war rooms |
| GET/PUT/DELETE | `/api/war-rooms/:id` | Get (nested groups + items) / update / delete a war room |
| POST | `/api/war-rooms/:id/groups` | Create a group in a war room |
| PUT/DELETE | `/api/war-room-groups/:id` | Update / delete a group |
| POST | `/api/war-room-groups/:id/items` | Create an item in a group |
| PUT/DELETE | `/api/war-room-items/:id` | Update / delete an item |
| GET/POST | `/api/calendars` | List / create calendar dashboards |
| GET/PUT/DELETE | `/api/calendars/:id` | Get (with components + events) / update / delete a calendar |
| POST | `/api/calendars/:id/components` | Create a component in a calendar |
| PUT/DELETE | `/api/calendar-components/:id` | Update / delete a component |
| POST | `/api/calendars/:id/events` | Create an event in a calendar |
| PUT/DELETE | `/api/calendar-events/:id` | Update / delete an event |
| GET | `/api/team-stats/summary` | KPIs, per-person debt, repo health and weekly trends (`?weeks&groups&repos`) |
| GET | `/api/team-stats/worklist` | Open PRs needing a reviewer (`?filter=unreviewed\|bot_only\|all`) |
| GET | `/api/team-stats/repos` | Repo registry with stats opt-in and per-repo sync state |
| POST | `/api/team-stats/sync` | Incremental sync of every tracked repo |
| POST | `/api/gh/execute` | Run a `gh` CLI command, returns JSON output (`cached: true` when served from disk) |
| POST | `/api/cache/invalidate` | Mark all current cache entries as invalidated (writes a timestamp; no files are deleted) |
| GET | `/api/cache/status` | Return current `cache_dir`, `ttl_secs`, and `invalidated_at` timestamp |
