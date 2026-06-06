-- A free-form Markdown "useful links" box per war room, pinned at the top of
-- the war room view when it has content.
ALTER TABLE war_rooms ADD COLUMN links TEXT NOT NULL DEFAULT '';
