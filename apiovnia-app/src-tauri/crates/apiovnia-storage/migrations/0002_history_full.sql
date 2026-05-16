-- Extend request_history with the columns we need to restore a full
-- ExecutionResult after an app restart: the outgoing request snapshot,
-- the final URL post-redirects, and the body kind / content-type. All
-- nullable so legacy rows keep working.

ALTER TABLE request_history ADD COLUMN sent_json    TEXT;
ALTER TABLE request_history ADD COLUMN final_url    TEXT;
ALTER TABLE request_history ADD COLUMN content_type TEXT;
ALTER TABLE request_history ADD COLUMN body_kind    TEXT;
