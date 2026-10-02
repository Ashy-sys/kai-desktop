CREATE TABLE IF NOT EXISTS kai_devices (
  device_id uuid PRIMARY KEY,
  token_hash text UNIQUE NOT NULL CHECK (length(token_hash) = 64),
  created_at timestamptz NOT NULL DEFAULT now(),
  last_seen_at timestamptz,
  revoked_at timestamptz
);

CREATE TABLE IF NOT EXISTS kai_registry_state (
  singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
  first_device_registered boolean NOT NULL DEFAULT false
);

INSERT INTO kai_registry_state (singleton, first_device_registered)
VALUES (true, false)
ON CONFLICT (singleton) DO NOTHING;

UPDATE kai_registry_state
SET first_device_registered = true
WHERE singleton = true
  AND EXISTS (SELECT 1 FROM kai_devices);
