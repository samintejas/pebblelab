-- MCP clients (Claude, ChatGPT, ...) connect with OAuth: they register themselves, the person approves them on the
-- hub, and they get a connector token. A client is public (no secret) and proves itself with PKCE.
CREATE TABLE oauth_clients (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    redirect_uris TEXT NOT NULL,                       -- one per line
    created_at    TEXT NOT NULL DEFAULT utc_now()
);

-- An approval waiting for the client to swap it for a token. Used once, within ten minutes.
CREATE TABLE oauth_codes (
    code_hash    TEXT PRIMARY KEY,
    client_id    TEXT NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    challenge    TEXT NOT NULL,
    scopes       TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT utc_now()
);

-- The client a connector token was issued to, so approving it again replaces the old token.
ALTER TABLE tokens ADD COLUMN client_id TEXT REFERENCES oauth_clients(id) ON DELETE SET NULL;
