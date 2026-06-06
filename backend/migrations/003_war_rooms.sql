-- War Rooms: a free-style, item-centric tracker for coordinated efforts
-- (e.g. patching a zero-day CVE across several repos and releases).
--
-- `repos` is a GLOBAL, reusable registry: define a friendly name + owner/repo
-- once (e.g. "Kgateway OSS" -> kgateway-dev/kgateway) and any war room can pick
-- it for a group. Items inherit their group's repo so gh commands can be
-- auto-scoped with `--repo owner/repo`.

CREATE TABLE IF NOT EXISTS repos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,              -- friendly label, e.g. "Kgateway OSS"
    owner_repo TEXT NOT NULL,        -- "kgateway-dev/kgateway"
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS war_rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'resolved', 'archived')),
    config TEXT NOT NULL DEFAULT '{}',   -- reserved for emoji map / report config (phase 3)
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS war_room_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    war_room_id INTEGER NOT NULL REFERENCES war_rooms(id) ON DELETE CASCADE,
    repo_id INTEGER REFERENCES repos(id) ON DELETE SET NULL,  -- registry origin (optional)
    name TEXT NOT NULL,              -- resolved/overridable group label
    repo TEXT,                       -- resolved owner/repo (self-contained copy)
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS war_room_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL REFERENCES war_room_groups(id) ON DELETE CASCADE,
    label TEXT NOT NULL,             -- "v2.3.2"
    ref_type TEXT CHECK(ref_type IN ('pr', 'issue') OR ref_type IS NULL),
    ref_number INTEGER,
    note TEXT NOT NULL DEFAULT '',   -- the "(envoy PR merged, released)" free text
    stage TEXT NOT NULL DEFAULT 'todo',  -- manual lifecycle (intentionally unconstrained)
    checklist TEXT NOT NULL DEFAULT '[]',-- JSON [{text, done}]
    depends_on INTEGER REFERENCES war_room_items(id) ON DELETE SET NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_war_room_groups_room ON war_room_groups(war_room_id);
CREATE INDEX IF NOT EXISTS idx_war_room_items_group ON war_room_items(group_id);
