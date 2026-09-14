-- Team Stats: locally-cached GitHub PR/review facts for opt-in repos, plus the
-- incremental-sync cursors.
--
-- Week buckets are Monday-start UTC dates produced by
--     date(ts, 'weekday 0', '-6 days')
-- which is correct for all seven weekdays and across the year boundary
-- (2026-01-01 -> 2025-12-29). Do NOT use date(ts,'weekday 1','-7 days'): it
-- returns the *previous* Monday when ts is already a Monday. Do NOT use
-- strftime('%Y-%W') (it emits a stub week '00') or %G-W%V (needs SQLite 3.46;
-- we run 3.44).

-- Opt-in per repo. Plain ADD COLUMN -- no CHECK constraint is involved, so
-- unlike migrations 011/012 this needs no table rebuild.
ALTER TABLE repos ADD COLUMN track_stats INTEGER NOT NULL DEFAULT 0;

-- One row per PR. NOT immutable: state / gh_updated_at / merged_at / is_draft
-- all change over a PR's life, so always upsert, never plain insert.
--
-- `repo` is the lowercased "owner/name" TEXT, deliberately NOT a FK to
-- repos(id): facts then survive un-tracking a repo, deleting and re-adding a
-- registry row, and a backup restore (which renumbers repos.id).
CREATE TABLE pr_facts (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    repo               TEXT    NOT NULL,           -- lower("owner/name")
    number             INTEGER NOT NULL,
    node_id            TEXT,
    title              TEXT    NOT NULL DEFAULT '',
    url                TEXT    NOT NULL DEFAULT '',
    state              TEXT    NOT NULL,           -- OPEN | CLOSED | MERGED
    is_draft           INTEGER NOT NULL DEFAULT 0,
    author_login       TEXT,                       -- NULL when the account is deleted
    merged_by_login    TEXT,
    -- gh_-prefixed so GitHub's timestamps are never confused with the
    -- house-style local created_at/updated_at every other table uses.
    gh_created_at      TEXT    NOT NULL,
    gh_updated_at      TEXT    NOT NULL,
    merged_at          TEXT,
    closed_at          TEXT,
    additions          INTEGER NOT NULL DEFAULT 0,
    deletions          INTEGER NOT NULL DEFAULT 0,
    changed_files      INTEGER NOT NULL DEFAULT 0,
    comment_count      INTEGER NOT NULL DEFAULT 0, -- issue comments only, NOT inline review comments
    review_total       INTEGER NOT NULL DEFAULT 0, -- reviews.totalCount as GitHub reported it
    reviews_truncated  INTEGER NOT NULL DEFAULT 0,
    requests_truncated INTEGER NOT NULL DEFAULT 0,
    created_week       TEXT GENERATED ALWAYS AS (date(gh_created_at, 'weekday 0', '-6 days')) STORED,
    merged_week        TEXT GENERATED ALWAYS AS (date(merged_at,     'weekday 0', '-6 days')) STORED,
    synced_at          TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (repo, number)
);

-- One row per submitted review event. `author_login` is the PR author,
-- denormalised so self-review exclusion is a column test rather than a join
-- (a PR's author never changes).
CREATE TABLE pr_review_facts (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    repo           TEXT    NOT NULL,
    number         INTEGER NOT NULL,
    node_id        TEXT,
    reviewer_login TEXT    NOT NULL,
    author_login   TEXT    NOT NULL,
    state          TEXT    NOT NULL,  -- APPROVED | CHANGES_REQUESTED | COMMENTED | DISMISSED
    submitted_at   TEXT    NOT NULL,
    is_self_review INTEGER GENERATED ALWAYS AS (reviewer_login = author_login COLLATE NOCASE) STORED,
    submitted_week TEXT    GENERATED ALWAYS AS (date(submitted_at, 'weekday 0', '-6 days')) STORED,
    synced_at      TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (repo, number, reviewer_login, submitted_at)
);

-- Currently-requested reviewers: a SNAPSHOT, not an event log. Replaced
-- wholesale per PR on every ingest. GitHub exposes no "requested at" timestamp
-- here, so this can answer "who is on the hook right now" but never "for how
-- long".
CREATE TABLE pr_review_requests (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    repo            TEXT NOT NULL,
    number          INTEGER NOT NULL,
    requested_login TEXT NOT NULL,        -- user login or team slug
    requested_type  TEXT NOT NULL CHECK(requested_type IN ('user','team')),
    synced_at       TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (repo, number, requested_type, requested_login)
);

CREATE TABLE stats_sync_state (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    repo             TEXT NOT NULL UNIQUE,
    -- Incremental watermark. Set to (run_start - 10 min), NEVER to
    -- max(updatedAt) -- see the comment at the assignment site in
    -- handlers/team_stats_sync.rs for why that would silently lose PRs.
    updated_cursor   TEXT,
    backfill_from    TEXT,                 -- '2026-08-01' by default
    backfilled       INTEGER NOT NULL DEFAULT 0,
    last_sync_at     TEXT,
    last_sync_status TEXT,                 -- 'ok' | 'partial' | 'error'
    last_error       TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
);

-- The ONE definition of "a real human review" and "this PR's review posture".
-- Both join team_members live, so filing a new AI reviewer under 'bot' in the
-- Team dialog retroactively corrects every historical metric with no re-sync.
-- That property is exactly why there is no rollup table and no stored
-- first_review_at column.
CREATE VIEW human_reviews AS
SELECT r.*
FROM pr_review_facts r
LEFT JOIN team_members tm ON tm.login = r.reviewer_login COLLATE NOCASE
WHERE r.is_self_review = 0
  AND COALESCE(tm.member_group, '') <> 'bot'
  AND r.reviewer_login NOT LIKE '%[bot]';

CREATE VIEW pr_stats AS
SELECT p.*,
  CASE WHEN p.author_login LIKE '%[bot]' THEN 'bot'
       ELSE COALESCE(tm.member_group, 'community') END AS author_group,
  (SELECT MIN(h.submitted_at) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number)                        AS first_human_review_at,
  (SELECT COUNT(*) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number)                        AS human_review_events,
  (SELECT COUNT(DISTINCT h.reviewer_login COLLATE NOCASE) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number)                        AS human_reviewers,
  (SELECT COUNT(*) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number AND h.state = 'APPROVED') AS human_approvals,
  -- A non-self review that is not a human review is a bot review. Defined by
  -- subtraction so the bot predicate is never restated.
  (SELECT COUNT(*) FROM pr_review_facts b
     WHERE b.repo = p.repo AND b.number = p.number AND b.is_self_review = 0
       AND b.id NOT IN (SELECT h.id FROM human_reviews h
                        WHERE h.repo = b.repo AND h.number = b.number))    AS bot_review_events,
  (SELECT COUNT(*) FROM pr_review_facts s
     WHERE s.repo = p.repo AND s.number = p.number AND s.is_self_review = 1) AS self_review_events
FROM pr_facts p
LEFT JOIN team_members tm ON tm.login = p.author_login COLLATE NOCASE;

CREATE INDEX idx_review_facts_reviewer_week ON pr_review_facts(reviewer_login COLLATE NOCASE, submitted_week);
CREATE INDEX idx_review_facts_week          ON pr_review_facts(submitted_week);
CREATE INDEX idx_review_requests_login      ON pr_review_requests(requested_login COLLATE NOCASE);
CREATE INDEX idx_pr_facts_author            ON pr_facts(author_login COLLATE NOCASE, gh_created_at);
CREATE INDEX idx_pr_facts_repo_state        ON pr_facts(repo, state, gh_created_at);
CREATE INDEX idx_pr_facts_created_week      ON pr_facts(created_week);
CREATE INDEX idx_pr_facts_open              ON pr_facts(is_draft, gh_created_at) WHERE state = 'OPEN';

-- Seed the known AI reviewers as bots.
--
-- This is not cosmetic. Measured across the five tracked repos on 2026-09-13,
-- `copilot-pull-request-reviewer` submitted 219 reviews -- more than any human,
-- and 32% of all non-self reviews. It carries no '[bot]' login suffix, so
-- neither the suffix heuristic nor the default roster catches it, and without
-- this seed the dashboard would ship reporting an AI as the team's top
-- reviewer: the exact distortion this feature exists to expose.
--
-- INSERT OR IGNORE respects idx_team_members_login, so a login the user has
-- already classified by hand keeps their classification.
INSERT OR IGNORE INTO team_members (login, member_group, source, position)
VALUES ('copilot-pull-request-reviewer', 'bot', 'builtin',
        (SELECT COALESCE(MAX(position) + 1, 0) FROM team_members));
