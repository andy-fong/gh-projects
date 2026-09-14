-- Stale-bot tracking.
--
-- In these repos a GitHub Actions workflow applies a `stale` label after a
-- period of inactivity and closes the PR a few days later if nothing happens.
-- Both halves are worth counting, and they are NOT the same number: a PR can be
-- labelled stale and then closed by a human, or revived and never closed.

-- Actor on the most recent CLOSED event. Raw login, so the bot predicate stays
-- a roster join like everywhere else rather than a hardcoded name.
ALTER TABLE pr_facts ADD COLUMN closed_by_login TEXT;

-- Most recent time the `stale` label was applied. One timestamp rather than an
-- event log: a PR that goes stale, is revived, and goes stale again counts only
-- in the later week. Re-staling is rare enough that an events table is not
-- worth the machinery.
ALTER TABLE pr_facts ADD COLUMN stale_labeled_at TEXT;

-- Whether it carries the `stale` label right now.
ALTER TABLE pr_facts ADD COLUMN is_stale INTEGER NOT NULL DEFAULT 0;

-- VIRTUAL, not STORED: SQLite only permits virtual generated columns in
-- ALTER TABLE ADD COLUMN. They are still indexable, which is all these need.
ALTER TABLE pr_facts ADD COLUMN stale_week TEXT
    GENERATED ALWAYS AS (date(stale_labeled_at, 'weekday 0', '-6 days')) VIRTUAL;
ALTER TABLE pr_facts ADD COLUMN closed_week TEXT
    GENERATED ALWAYS AS (date(closed_at, 'weekday 0', '-6 days')) VIRTUAL;

CREATE INDEX idx_pr_facts_stale_week  ON pr_facts(stale_week);
CREATE INDEX idx_pr_facts_closed_week ON pr_facts(closed_week);
CREATE INDEX idx_pr_facts_stale       ON pr_facts(is_stale) WHERE is_stale = 1;

-- The stale workflow runs as `github-actions`. Seeding it as a bot means the
-- existing roster-join predicate identifies it, and it also keeps any review it
-- leaves out of the human review counts. Idempotent, and a hand classification
-- already on the roster wins.
INSERT OR IGNORE INTO team_members (login, member_group, source, position)
VALUES ('github-actions', 'bot', 'builtin',
        (SELECT COALESCE(MAX(position) + 1, 0) FROM team_members));

-- Recreate the view so the new columns flow through with a derived
-- "the bot closed this" flag. `SELECT p.*` already picks up the raw columns.
DROP VIEW IF EXISTS pr_stats;
CREATE VIEW pr_stats AS
SELECT p.*,
  CASE WHEN p.author_login LIKE '%[bot]' THEN 'bot'
       ELSE COALESCE(tm.member_group, 'community') END AS author_group,
  -- Closed, never merged, carrying the stale label, and closed by a bot. All
  -- four matter: a human closing a stale PR is a decision, not automation.
  CASE WHEN p.state = 'CLOSED' AND p.merged_at IS NULL AND p.is_stale = 1
            AND (p.closed_by_login LIKE '%[bot]'
                 OR LOWER(p.closed_by_login) IN
                    (SELECT LOWER(login) FROM team_members WHERE member_group = 'bot'))
       THEN 1 ELSE 0 END AS closed_by_stale_bot,
  (SELECT MIN(h.submitted_at) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number)                        AS first_human_review_at,
  (SELECT COUNT(*) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number)                        AS human_review_events,
  (SELECT COUNT(DISTINCT h.reviewer_login COLLATE NOCASE) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number)                        AS human_reviewers,
  (SELECT COUNT(*) FROM human_reviews h
     WHERE h.repo = p.repo AND h.number = p.number AND h.state = 'APPROVED') AS human_approvals,
  (SELECT COUNT(*) FROM pr_review_facts b
     WHERE b.repo = p.repo AND b.number = p.number AND b.is_self_review = 0
       AND b.id NOT IN (SELECT h.id FROM human_reviews h
                        WHERE h.repo = b.repo AND h.number = b.number))    AS bot_review_events,
  (SELECT COUNT(*) FROM pr_review_facts s
     WHERE s.repo = p.repo AND s.number = p.number AND s.is_self_review = 1) AS self_review_events
FROM pr_facts p
LEFT JOIN team_members tm ON tm.login = p.author_login COLLATE NOCASE;
