create table settings (
    id integer primary key not null default 1 check (id = 1),
    visitor_group_mode text not null,
    track_sessions boolean not null,
    track_utm_params boolean not null,
    track_geo text not null,
    history_days integer,
    ingest_filters_json text not null default '[]'
);

insert into settings (
    id,
    visitor_group_mode,
    track_sessions,
    track_utm_params,
    track_geo,
    history_days,
    ingest_filters_json
) values (1, 'accurate', true, true, 'city', null, '[]');

create table entity_settings (
    entity_id text primary key not null,
    visitor_group_mode text,
    track_sessions boolean,
    track_utm_params boolean,
    track_geo text,
    history_mode text not null default 'inherit',
    history_days integer,
    ingest_filters_json text not null default '[]',
    foreign key (entity_id) references entities(id) on delete cascade
);

create table project_settings (
    project_id text primary key not null,
    metric_display_overrides_json text not null default '{}',
    dimension_display_overrides_json text not null default '{}',
    foreign key (project_id) references projects(id) on delete cascade
);
