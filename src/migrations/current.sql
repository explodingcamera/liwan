CREATE SEQUENCE sessions_id_seq;
CREATE SEQUENCE migrations_id_seq;

-- TODO: Use FSST for URLs
--       Use Frame of Reference encoding for timestamps
--       Use Dictionary Encoding for entity_id, event, and metadata 

CREATE TABLE events (
    entity_id TEXT NOT NULL,
    visitor_id TEXT NOT NULL,
    event TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- metadata
    fqdn TEXT,
    path TEXT,
    referrer TEXT,
    platform TEXT,
    browser TEXT,
    mobile BOOLEAN,
    country TEXT,
    city TEXT,
    -- currently unsupported by the rust driver: meta MAP(TEXT, TEXT),
);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY DEFAULT nextval('sessions_id_seq'),
    token TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL
);

CREATE TABLE migrations (
    id INTEGER PRIMARY KEY DEFAULT nextval('migrations_id_seq'), 
    name TEXT NOT NULL
);

CREATE TABLE salts (
    id INTEGER PRIMARY KEY DEFAULT 1,
    salt TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
 
INSERT INTO migrations (name) VALUES ('2024-06-01-initial');
INSERT INTO salts (salt, updated_at) VALUES ('', '1970-01-01 00:00:00');
CREATE INDEX sessions_token_idx ON sessions (token);
CREATE INDEX events_entity_id_idx ON events (entity_id);
