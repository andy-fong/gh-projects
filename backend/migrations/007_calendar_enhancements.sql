ALTER TABLE calendar_components ADD COLUMN short_name TEXT DEFAULT NULL;
ALTER TABLE calendar_events ADD COLUMN actual_release_date TEXT DEFAULT NULL;
