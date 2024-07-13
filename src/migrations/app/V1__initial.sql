create table users (
    username text primary key not null,
    password_hash text not null,
    role text not null,
    projects text not null -- comma separated list of project ids
);

create table sessions (
    id  text primary key not null,
    expires_at timestamp not null,
    username text not null
);
 
create table entities (
    id text primary key not null,
    display_name text not null
);

create table projects (
    id text primary key not null,
    display_name text not null,
    public boolean not null,
    secret text
);

create table project_entities (
    project_id text not null,
    entity_id text not null,
    primary key (project_id, entity_id)
);

create table salts (
    id integer primary key default 1,
    salt text not null,
    updated_at timestamp not null default (datetime('now'))
);

insert into salts (id, salt, updated_at) values (1, '', '1970-01-01 00:00:00');