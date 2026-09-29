create table teams (
    id text primary key not null,
    display_name text not null,
    all_entities integer not null default 0,
    all_projects integer not null default 0,
    permissions_json text not null default '["project:read"]'
);

create table team_users (
    team_id text not null,
    username text not null,
    primary key (team_id, username)
);

create index team_users_username on team_users (username);

create table team_projects (
    team_id text not null,
    project_id text not null,
    primary key (team_id, project_id)
);

create table team_entities (
    team_id text not null,
    entity_id text not null,
    primary key (team_id, entity_id)
);

alter table users drop column projects;
alter table projects add column visibility text not null default 'private' check (visibility in ('private', 'public', 'unlisted', 'internal'));
update projects set visibility = case when public and unlisted then 'unlisted' when public then 'public' else 'private' end;
alter table projects drop column public;
alter table projects drop column unlisted;
