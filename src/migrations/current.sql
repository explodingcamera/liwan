
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
    id  TEXT PRIMARY KEY NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    data TEXT NOT NULL,
);

CREATE SEQUENCE migrations_id_seq;
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

CREATE INDEX events_event_idx ON events (event);
CREATE INDEX events_entity_id_idx ON events (entity_id);
CREATE INDEX events_visitor_id_idx ON events (visitor_id);


SET enable_fsst_vectors = true;