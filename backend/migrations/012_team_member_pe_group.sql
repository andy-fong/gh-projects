-- Add a 'pe' group for the Product Excellence team. As with 011, SQLite can't
-- alter a CHECK constraint in place, so both tables are rebuilt; nothing
-- references them, and existing rows keep their ids.

CREATE TABLE team_members_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    login TEXT NOT NULL,
    member_group TEXT NOT NULL CHECK(member_group IN ('team','pe','solo','maintainer','bot')),
    source TEXT NOT NULL DEFAULT 'manual',
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
INSERT INTO team_members_new (id, login, member_group, source, position, created_at, updated_at)
    SELECT id, login, member_group, source, position, created_at, updated_at FROM team_members;
DROP TABLE team_members;
ALTER TABLE team_members_new RENAME TO team_members;
CREATE UNIQUE INDEX IF NOT EXISTS idx_team_members_login
    ON team_members(login COLLATE NOCASE);

CREATE TABLE team_member_sources_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    member_group TEXT NOT NULL CHECK(member_group IN ('team','pe','solo','maintainer','bot')),
    org_team TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
INSERT INTO team_member_sources_new (id, member_group, org_team, position, created_at, updated_at)
    SELECT id, member_group, org_team, position, created_at, updated_at FROM team_member_sources;
DROP TABLE team_member_sources;
ALTER TABLE team_member_sources_new RENAME TO team_member_sources;
