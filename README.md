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

This compiles the Dioxus app to `target/dx/gh-projects-web/release/web/public/`.
(On some macOS setups `wasm-opt` aborts; `dx` falls back to an unoptimized but
fully working bundle — that's fine.)

> **Rebuilding / changes not showing up?** `dx build` writes content-hashed
> assets into the output directory but doesn't prune old ones, so they pile up
> across builds. `index.html` itself isn't cache-busted, so after a rebuild your
> browser may still load the previous bundle — **hard-refresh** (Cmd+Shift+R) to
> pick up changes. To start from a clean bundle, delete the output directory
> first: `rm -rf target/dx/gh-projects-web/release/web/public`.

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
| `STATIC_DIR` | `target/dx/gh-projects-web/release/web/public` | Directory of the built frontend bundle to serve |

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
| POST | `/api/gh/execute` | Run a `gh` CLI command, returns JSON output (`cached: true` when served from disk) |
| POST | `/api/cache/invalidate` | Mark all current cache entries as invalidated (writes a timestamp; no files are deleted) |
| GET | `/api/cache/status` | Return current `cache_dir`, `ttl_secs`, and `invalidated_at` timestamp |
