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

-- todo: evaluate if these indexes are necessary
create index events_event_idx on events (event);
create index events_entity_id_idx on events (entity_id);
create index events_visitor_id_idx on events (visitor_id);
create index events_created_at_idx on events (created_at);
create index events_entity_id_created_at_idx on events (entity_id, created_at);
create index events_visitor_id_created_at_idx on events (visitor_id, created_at);
