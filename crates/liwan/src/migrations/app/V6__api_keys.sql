create table api_keys (
    id text primary key not null,
    secret_hash text not null unique,
    display_name text not null,
    permissions_json text not null,
    all_entities integer not null default 0,
    all_projects integer not null default 0,
    expires_at timestamp,
    created_at timestamp not null,
    last_used_at timestamp
);

create table api_key_entities (
    key_id text not null,
    entity_id text not null,
    primary key (key_id, entity_id)
);

create index api_key_entities_entity_id on api_key_entities (entity_id);

create table api_key_projects (
    key_id text not null,
    project_id text not null,
    primary key (key_id, project_id)
);
