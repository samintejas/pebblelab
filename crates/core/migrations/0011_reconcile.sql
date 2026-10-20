-- Reconciling an account against its statement closes everything up to that date. `reconciled_through` is the
-- statement's date and `reconciled_balance` the balance it showed (as shown in the app: owed, for a card). Nothing
-- on or before that date may change the balance again, so a mistake found later is corrected with a new
-- transaction dated after it. Descriptions, notes, tags and files stay editable.
ALTER TABLE accounts ADD COLUMN reconciled_through TEXT;
ALTER TABLE accounts ADD COLUMN reconciled_balance BIGINT;

-- The rule lives in the database so every way in (the app, the api, mcp, the cli, renewals) keeps it.
-- SQLSTATE PBL01 is turned into a plain refusal by the server.
CREATE FUNCTION tx_reconciled_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    acct BIGINT;
    day  TEXT;
    shut TEXT;
    nm   TEXT;
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.amount = OLD.amount AND NEW.date = OLD.date AND NEW.account_id = OLD.account_id THEN
        RETURN NEW;
    END IF;
    FOR acct, day IN
        SELECT * FROM (VALUES (CASE WHEN TG_OP <> 'INSERT' THEN OLD.account_id END, CASE WHEN TG_OP <> 'INSERT' THEN OLD.date END),
                              (CASE WHEN TG_OP <> 'DELETE' THEN NEW.account_id END, CASE WHEN TG_OP <> 'DELETE' THEN NEW.date END)) v
        WHERE v.column1 IS NOT NULL
    LOOP
        -- FOR SHARE waits for a reconcile in progress; an account being deleted is already gone, so its rows go
        SELECT reconciled_through, name INTO shut, nm FROM accounts WHERE id = acct FOR SHARE;
        IF shut IS NOT NULL AND day <= shut THEN
            RAISE EXCEPTION '% is reconciled through %, so nothing on or before that date can change. add a correction dated after it instead', nm, shut
                USING ERRCODE = 'PBL01';
        END IF;
    END LOOP;
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;

CREATE TRIGGER tx_reconciled_guard BEFORE INSERT OR UPDATE OR DELETE ON transactions
    FOR EACH ROW EXECUTE FUNCTION tx_reconciled_guard();

-- The opening figure is part of every past balance, so it is fixed while an account is reconciled.
CREATE FUNCTION account_reconciled_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.reconciled_through IS NOT NULL AND NEW.reconciled_through IS NOT DISTINCT FROM OLD.reconciled_through AND NEW.opening <> OLD.opening THEN
        RAISE EXCEPTION '% is reconciled through %, so its opening balance is fixed', OLD.name, OLD.reconciled_through
            USING ERRCODE = 'PBL01';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER account_reconciled_guard BEFORE UPDATE OF opening ON accounts
    FOR EACH ROW EXECUTE FUNCTION account_reconciled_guard();
