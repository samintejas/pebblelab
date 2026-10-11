-- The account hub (pebblelab.in): which app opened a session, and a provider sign-in that is adding a way to
-- sign in to an account that already exists, not signing in.
ALTER TABLE tokens ADD COLUMN app TEXT NOT NULL DEFAULT '';
ALTER TABLE oauth_flows ADD COLUMN link_user BIGINT REFERENCES users(id) ON DELETE CASCADE;
