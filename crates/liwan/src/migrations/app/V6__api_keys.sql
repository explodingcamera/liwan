create table api_keys (
    id text primary key not null,
    display_name text not null,
    permissions_json text not null,
    created_at timestamp not null,
    last_used_at timestamp,
    revoked_at timestamp
);

create table api_key_entities (
    key_id text not null,
    entity_id text not null,
    primary key (key_id, entity_id)
);

create index api_key_entities_entity_id on api_key_entities (entity_id);
