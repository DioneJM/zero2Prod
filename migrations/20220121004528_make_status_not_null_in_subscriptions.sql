-- Add migration script here

-- Wrap whole migration in transaction so that the migration can only succeed atomically
BEGIN;
    UPDATE subscriptions
        SET status = 'confirmed'
        WHERE status IS NULL;
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL; -- make status mandatory
COMMIT;