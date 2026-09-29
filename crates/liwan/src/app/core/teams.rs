use anyhow::{Result, bail};
use rand::distr::{Alphanumeric, SampleString};
use std::collections::HashSet;

use crate::app::{
    SqlitePool,
    models::{Access, AccessPermission, AccessScope, Team},
};

#[derive(Clone)]
pub struct LiwanTeams {
    pool: SqlitePool,
}

impl LiwanTeams {
    /// Create a team store.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// List teams with their members and granted projects.
    pub fn all(&self) -> Result<Vec<Team>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "select id, display_name, all_entities, all_projects, permissions_json,
                (select json_group_array(username) from team_users where team_id = teams.id),
                (select json_group_array(project_id) from team_projects where team_id = teams.id),
                (select json_group_array(entity_id) from team_entities where team_id = teams.id)
             from teams order by display_name, id",
        )?;
        let teams = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        let mut result = Vec::new();
        for team in teams {
            let (id, display_name, all_entities, all_projects, permissions, users, projects, entities) = team?;
            result.push(Team {
                id,
                display_name,
                users: serde_json::from_str(&users)?,
                access: Access {
                    entities: if all_entities {
                        AccessScope::All
                    } else {
                        AccessScope::Selected(serde_json::from_str(&entities)?)
                    },
                    projects: if all_projects {
                        AccessScope::All
                    } else {
                        AccessScope::Selected(serde_json::from_str(&projects)?)
                    },
                    permissions: serde_json::from_str::<HashSet<AccessPermission>>(&permissions)?,
                },
            });
        }
        Ok(result)
    }

    /// Create a team with a stable ID.
    pub fn create(&self, display_name: &str) -> Result<String> {
        if display_name.trim().is_empty() {
            bail!("team name cannot be empty");
        }
        let id = Alphanumeric.sample_string(&mut rand::rng(), 16);
        self.pool.get()?.execute("insert into teams (id, display_name) values (?, ?)", (&id, display_name.trim()))?;
        Ok(id)
    }

    /// Replace a team's name, members, and project grants atomically.
    pub fn update(
        &self,
        id: &str,
        display_name: &str,
        users: &[String],
        projects: &AccessScope,
        entities: Option<&AccessScope>,
    ) -> Result<bool> {
        if display_name.trim().is_empty() {
            bail!("team name cannot be empty");
        }
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        let all_projects = matches!(projects, AccessScope::All);
        if tx.execute(
            "update teams set display_name = ?, all_projects = ? where id = ?",
            rusqlite::params![display_name.trim(), all_projects, id],
        )? == 0
        {
            return Ok(false);
        }
        for username in users {
            if !tx.prepare_cached("select 1 from users where username = ?")?.exists([username])? {
                bail!("user not found: {username}");
            }
        }
        let selected_projects = match projects {
            AccessScope::All => &[][..],
            AccessScope::Selected(ids) => ids.as_slice(),
        };
        for project_id in selected_projects {
            if !tx.prepare_cached("select 1 from projects where id = ?")?.exists([project_id])? {
                bail!("project not found: {project_id}");
            }
        }
        if let Some(AccessScope::Selected(ids)) = entities {
            for entity_id in ids {
                if !tx.prepare_cached("select 1 from entities where id = ?")?.exists([entity_id])? {
                    bail!("entity not found: {entity_id}");
                }
            }
        }
        tx.execute("delete from team_users where team_id = ?", [id])?;
        tx.execute("delete from team_projects where team_id = ?", [id])?;
        for username in users {
            tx.execute("insert or ignore into team_users (team_id, username) values (?, ?)", (id, username))?;
        }
        for project_id in selected_projects {
            tx.execute("insert or ignore into team_projects (team_id, project_id) values (?, ?)", (id, project_id))?;
        }
        if let Some(entities) = entities {
            tx.execute(
                "update teams set all_entities = ? where id = ?",
                rusqlite::params![matches!(entities, AccessScope::All), id],
            )?;
            tx.execute("delete from team_entities where team_id = ?", [id])?;
            if let AccessScope::Selected(ids) = entities {
                for entity_id in ids {
                    tx.execute(
                        "insert or ignore into team_entities (team_id, entity_id) values (?, ?)",
                        (id, entity_id),
                    )?;
                }
            }
        }
        tx.commit()?;
        Ok(true)
    }

    /// Delete a team and revoke its grants.
    pub fn delete(&self, id: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let tx = conn.transaction()?;
        tx.execute("delete from team_users where team_id = ?", [id])?;
        tx.execute("delete from team_projects where team_id = ?", [id])?;
        tx.execute("delete from team_entities where team_id = ?", [id])?;
        tx.execute("delete from teams where id = ?", [id])?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{
        Liwan,
        models::{AccessPermission, AccessScope, Entity, Project, ProjectVisibility, UserRole},
    };
    use crate::config::Config;
    use crate::utils::validate::{can_enumerate_project, can_view_project};

    #[test]
    fn team_grants_follow_membership_and_project_changes() {
        let app = Liwan::new_memory(Config::default()).unwrap();
        app.users.create("alice", "password", UserRole::User).unwrap();
        let project = Project {
            id: "private".into(),
            display_name: "Private".into(),
            visibility: ProjectVisibility::Private,
            secret: None,
        };
        app.projects.create(&project, &[]).unwrap();
        let other = Project { id: "other".into(), ..project.clone() };
        app.projects.create(&other, &[]).unwrap();
        app.entities.create(&Entity { id: "site".into(), display_name: "Site".into() }, &[]).unwrap();
        assert!(app.teams.create(" ").is_err());
        let editors_id = app.teams.create("Editors").unwrap();
        let session = "test-session";
        app.sessions.create(session, "alice", chrono::Utc::now() + chrono::Duration::hours(1)).unwrap();
        assert!(!can_view_project(&project, app.sessions.get(session).unwrap().as_ref()));
        app.teams
            .update(
                &editors_id,
                "Reviewers",
                &["alice".into()],
                &AccessScope::Selected(vec![]),
                Some(&AccessScope::Selected(vec!["site".into()])),
            )
            .unwrap();
        assert!(matches!(&app.teams.all().unwrap()[0].access.entities, AccessScope::Selected(ids) if ids == &["site"]));
        let scoped_user = app.sessions.get(session).unwrap().unwrap();
        assert!(matches!(&scoped_user.access.entities, AccessScope::Selected(ids) if ids == &["site"]));
        assert!(!scoped_user.can_read_entity("site"));
        assert!(!can_view_project(&project, Some(&scoped_user)));

        app.teams
            .update(&editors_id, "Reviewers", &["alice".into()], &AccessScope::Selected(vec!["private".into()]), None)
            .unwrap();
        let user = app.sessions.get(session).unwrap().unwrap();
        assert!(can_view_project(&project, Some(&user)));
        assert!(!can_view_project(&other, Some(&user)));
        assert!(matches!(user.access.projects, AccessScope::Selected(_)));
        assert!(can_enumerate_project(&project, Some(&user)));
        assert_eq!(app.teams.all().unwrap()[0].display_name, "Reviewers");
        assert!(app.teams.all().unwrap()[0].access.permissions.contains(&AccessPermission::ProjectRead));
        let auditors_id = app.teams.create("Auditors").unwrap();
        app.teams
            .update(&auditors_id, "Auditors", &["alice".into()], &AccessScope::Selected(vec!["private".into()]), None)
            .unwrap();
        assert!(
            matches!(&app.sessions.get(session).unwrap().unwrap().access.projects, AccessScope::Selected(ids) if ids == &["private"])
        );

        assert!(
            app.teams
                .update(&editors_id, "Reviewers", &["missing".into()], &AccessScope::Selected(vec![]), None)
                .is_err()
        );
        assert!(can_view_project(&project, app.sessions.get(session).unwrap().as_ref()));
        app.teams
            .update(&editors_id, "Reviewers", &["alice".into()], &AccessScope::All, Some(&AccessScope::All))
            .unwrap();
        let editors = app.teams.all().unwrap().into_iter().find(|team| team.id == editors_id).unwrap();
        assert!(matches!(editors.access.projects, AccessScope::All));
        assert!(matches!(editors.access.entities, AccessScope::All));
        let all_user = app.sessions.get(session).unwrap().unwrap();
        assert!(matches!(all_user.access.projects, AccessScope::All));
        assert!(matches!(all_user.access.entities, AccessScope::All));
        assert!(can_view_project(&other, Some(&all_user)));
        app.teams.update(&editors_id, "Reviewers", &["alice".into()], &AccessScope::Selected(vec![]), None).unwrap();
        let editors = app.teams.all().unwrap().into_iter().find(|team| team.id == editors_id).unwrap();
        assert!(matches!(editors.access.entities, AccessScope::All));
        assert!(can_view_project(&project, app.sessions.get(session).unwrap().as_ref()));
        assert!(!can_view_project(&other, app.sessions.get(session).unwrap().as_ref()));
        app.teams.delete(&auditors_id).unwrap();
        assert!(!can_view_project(&project, app.sessions.get(session).unwrap().as_ref()));
        app.teams.delete(&editors_id).unwrap();
        assert!(!can_view_project(&project, app.sessions.get(session).unwrap().as_ref()));
    }
}
