-- A monthly limit on one tag. What counts against it is the money out, less refunds, of transactions with that tag
-- (in any place, not only the first) on the accounts the person can see. Transfers never count.
CREATE TABLE budgets (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tag        TEXT NOT NULL CHECK (tag <> ''),
    amount     BIGINT NOT NULL CHECK (amount > 0),
    created_at TEXT NOT NULL DEFAULT utc_now(),
    UNIQUE (user_id, tag)
);
