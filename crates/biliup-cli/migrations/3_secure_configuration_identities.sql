-- A Web login has exactly one administrator identity.  Creating the partial
-- unique index intentionally fails when an existing database already contains
-- multiple `biliup` rows: choosing one automatically could preserve an
-- attacker-created password, so startup must fail closed until the operator
-- resolves the conflicting rows.
CREATE UNIQUE INDEX IF NOT EXISTS uq_configuration_biliup_identity
    ON configuration (key)
    WHERE key = 'biliup';

-- Duplicate references to the exact same cookie file are semantically
-- equivalent and can be collapsed safely before enforcing atomic idempotency.
DELETE FROM configuration
WHERE key = 'bilibili-cookies'
  AND id NOT IN (
      SELECT MIN(id)
      FROM configuration
      WHERE key = 'bilibili-cookies'
      GROUP BY value
  );

CREATE UNIQUE INDEX IF NOT EXISTS uq_configuration_bilibili_cookie
    ON configuration (key, value)
    WHERE key = 'bilibili-cookies';
