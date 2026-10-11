-- A public id for each transaction that is unambiguous to share, unlike the row number.
ALTER TABLE transactions ADD COLUMN uid UUID NOT NULL DEFAULT gen_random_uuid();
CREATE UNIQUE INDEX transactions_uid ON transactions (uid);
