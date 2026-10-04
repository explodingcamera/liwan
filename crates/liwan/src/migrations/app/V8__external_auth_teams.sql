alter table external_auth_settings add column default_team_id text;
alter table external_auth_settings add column group_team_mappings text not null default '[]';
alter table external_auth_settings add column additional_scopes text not null default '';
alter table external_auth_settings add column group_claim_name text not null default '';
