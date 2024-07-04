create table app.sessions (
    id  text primary key not null,
    expires_at timestamp not null,
    data text not null,
);

create table app.salts (
    id integer primary key default 1,
    salt text not null,
    updated_at timestamp not null default now()
);

insert into app.salts (salt, updated_at) values ('', '1970-01-01 00:00:00');
