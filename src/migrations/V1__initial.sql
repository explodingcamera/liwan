SET enable_fsst_vectors = true;

create table events (
    entity_id text not null,
    visitor_id text not null,
    event text not null,
    created_at timestamp not null default now(),

    -- metadata
    fqdn text,
    path text,
    referrer text,
    platform text,
    browser text,
    mobile boolean,
    country text,
    city text,
    -- currently unsupported by the rust driver: meta map(text, text),
);

create table sessions (
    id  text primary key not null,
    expires_at timestamp not null,
    data text not null,
);

create table salts (
    id integer primary key default 1,
    salt text not null,
    updated_at timestamp not null default now()
);

insert into salts (salt, updated_at) values ('', '1970-01-01 00:00:00');

-- todo: evaluate if these indexes are necessary
create index events_event_idx on events (event);
create index events_entity_id_idx on events (entity_id);
create index events_visitor_id_idx on events (visitor_id);
create index events_created_at_idx on events (created_at);
create index events_entity_id_created_at_idx on events (entity_id, created_at);
create index events_visitor_id_created_at_idx on events (visitor_id, created_at);
