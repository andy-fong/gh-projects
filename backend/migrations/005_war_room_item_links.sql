-- Linked items: a reference row in one group that mirrors a "source" item
-- owned by another group. The reference carries `source_item_id`; its own
-- content fields are unused (the frontend renders the source's live data).
-- ON DELETE CASCADE: deleting the source item removes all of its mirrors.
ALTER TABLE war_room_items
    ADD COLUMN source_item_id INTEGER REFERENCES war_room_items(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_war_room_items_source ON war_room_items(source_item_id);
