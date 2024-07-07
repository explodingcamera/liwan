CREATE TYPE app.user_role AS ENUM ('admin', 'user');

create table app.users (
    username text primary key not null,
    password_hash text not null,
    role user_role not null,
    projects text not null, -- comma separated list of project ids, replace with list once that's supported by duckdb.rs
);

create table app.sessions (
    id  text primary key not null,
    expires_at timestamp not null,
    username text not null, -- foreign keys seem to be broken for non-main tables
);
 
create table app.entities (
    id text primary key not null,
    display_name text not null,
);

create table app.projects (
    id text primary key not null,
    display_name text not null,
    public boolean not null,
    secret text not null,
);

create table app.project_entities (
    project_id text not null,
    entity_id text not null,
    primary key (project_id, entity_id),
);

create table app.salts (
    id integer primary key default 1,
    salt text not null,
    updated_at timestamp not null default now()
);

insert into app.salts (salt, updated_at) values ('', '1970-01-01 00:00:00');
