# GH Projects

A custom GitHub dashboard — Grafana-style drag/resize tiles where each tile runs a `gh` CLI command to fetch live data, plus private notes tied to GitHub issues and PRs stored locally in SQLite.

## Features

- **Dashboards** — create multiple named dashboards, each with an independent tile grid
- **Drag & resize tiles** — powered by react-grid-layout; layouts are persisted to the database
- **GH Query tiles** — each tile runs any `gh` CLI command and displays the JSON result as a table
- **Note tiles** — embed a private note directly on a dashboard
- **Notes page** — full CRUD for private Markdown notes, optionally linked to a GitHub repo / issue / PR; one-click link to open the issue or PR on GitHub

## Prerequisites

- Rust toolchain (`cargo`)
- Node.js 18+
- `gh` CLI authenticated (`gh auth login`)

## Running

### Backend

```bash
cd /path/to/gh-projects
cargo run -p gh-projects-backend
```

The server starts on **http://localhost:3001**. The SQLite database file `gh-projects.db` is created automatically in the directory where you run the command.

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

### Frontend

```bash
cd frontend
npm install
npm run dev
```

The dev server starts on **http://localhost:5173** and proxies `/api` requests to the backend.

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

## Project structure

```
gh-projects/
├── Cargo.toml              # workspace
├── backend/                # Rust + Axum + sqlx
│   ├── migrations/         # SQL migrations (run automatically on startup)
│   └── src/
│       ├── main.rs
│       ├── handlers/       # HTTP route handlers
│       ├── models/         # Serde structs
│       ├── repositories/   # DB access (trait + SQLite impl)
│       └── state.rs        # Shared app state
└── frontend/               # React + Vite + Tailwind
    └── src/
        ├── api/            # Typed API client
        ├── components/     # Sidebar, TileWrapper, tiles, dialogs
        ├── pages/          # DashboardPage, NotesPage
        └── types/          # TypeScript interfaces
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
| POST | `/api/gh/execute` | Run a `gh` CLI command, returns JSON output (`cached: true` when served from disk) |
| POST | `/api/cache/invalidate` | Mark all current cache entries as invalidated (writes a timestamp; no files are deleted) |
| GET | `/api/cache/status` | Return current `cache_dir`, `ttl_secs`, and `invalidated_at` timestamp |
