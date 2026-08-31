alter table users rename to users_old;

create table users (
    username text primary key not null,
    password_hash text,
    role text not null,
    projects text not null
);

insert into users (username, password_hash, role, projects)
select username, password_hash, role, projects from users_old;

drop table users_old;

create table external_auth_settings (
    id integer primary key not null default 1 check (id = 1),
    enabled boolean not null default false check (enabled in (false, true)),
    provider text not null default 'oidc',
    display_name text not null default 'OpenID Connect',
    client_id text not null default '',
    client_secret text,
    issuer_url text,
    allowed_domain text,
    tenant_id text,
    allow_user_creation boolean not null default false check (allow_user_creation in (false, true)),
    allow_session_reuse boolean not null default true check (allow_session_reuse in (false, true))
);

insert into external_auth_settings (id) values (1);

create table external_identities (
    provider_key text not null check (length(provider_key) > 0),
    subject text not null check (length(subject) > 0),
    username text not null,
    primary key (provider_key, subject),
    unique (provider_key, username),
    foreign key (username) references users(username) on delete cascade
);
