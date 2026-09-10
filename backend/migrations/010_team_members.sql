-- Roster of GitHub logins used to classify item authors in GH Query tiles.
-- `community` is deliberately absent: it is the fallback for any login that
-- is not in this table.
CREATE TABLE IF NOT EXISTS team_members (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    login TEXT NOT NULL,
    member_group TEXT NOT NULL CHECK(member_group IN ('team','maintainer','bot')),
    source TEXT NOT NULL DEFAULT 'manual',
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- One group per login, case-insensitive: GitHub logins are not case sensitive.
CREATE UNIQUE INDEX IF NOT EXISTS idx_team_members_login
    ON team_members(login COLLATE NOCASE);

-- Where a group can re-import its logins from. `org_team` is either
-- "owner/team-slug" (a GitHub team) or a bare "owner" (whole org).
CREATE TABLE IF NOT EXISTS team_member_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    member_group TEXT NOT NULL CHECK(member_group IN ('team','maintainer','bot')),
    org_team TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
