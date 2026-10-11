-- pebblelab becomes an account with apps under it.
--
-- A session belongs to the account or to one app. An app session is opened by an account session handing the
-- browser over, and ends with it. Sessions from before this are account sessions, so nobody is signed out.
ALTER TABLE tokens ADD COLUMN app TEXT NOT NULL DEFAULT 'account';
ALTER TABLE tokens ADD COLUMN parent_id BIGINT REFERENCES tokens(id) ON DELETE CASCADE;
CREATE INDEX idx_tokens_parent ON tokens(parent_id);

-- Connector scopes name their app: `read` is now `fin:read`.
UPDATE tokens SET scopes = regexp_replace(scopes, '(^|,)([a-z]+)(?=,|$)', '\1fin:\2', 'g') WHERE kind = 'connector';
ALTER TABLE tokens ALTER COLUMN scopes SET DEFAULT 'fin:read,fin:transactions,fin:add,fin:edit';

-- A one-time code says which app it signs in to, and which account session asked for it.
ALTER TABLE login_codes ADD COLUMN app TEXT NOT NULL DEFAULT 'account';
ALTER TABLE login_codes ADD COLUMN parent_id BIGINT REFERENCES tokens(id) ON DELETE CASCADE;

-- Family lives in the account now.
UPDATE notifications SET link = 'account/family' WHERE link = 'settings/family';
