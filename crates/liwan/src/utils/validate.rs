use crate::app::models::{Project, ProjectVisibility, User};

pub fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

pub fn is_valid_username(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && name.len() <= 64 && name.len() >= 2
}

pub fn can_view_project(project: &Project, user: Option<&User>) -> bool {
    matches!(project.visibility, ProjectVisibility::Public | ProjectVisibility::Unlisted)
        || (project.visibility == ProjectVisibility::Internal && user.is_some())
        || user.is_some_and(|u| u.can_read_project(&project.id))
}

pub fn can_enumerate_project(project: &Project, user: Option<&User>) -> bool {
    can_view_project(project, user)
        && (project.visibility != ProjectVisibility::Unlisted || user.is_some_and(|u| u.can_read_project(&project.id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{Access, AccessPermission, AccessScope, Project, ProjectVisibility, User, UserRole};

    #[test]
    fn test_project_visibility() {
        let project = Project {
            id: "public_project".to_string(),
            display_name: "Public Project".to_string(),
            secret: None,
            visibility: ProjectVisibility::Public,
        };
        assert!(can_view_project(&project, None), "Public project should be accessible without a user.");
        assert!(can_enumerate_project(&project, None), "Public listed project should be enumerable.");

        let user = User { username: "test".to_string(), role: UserRole::User, access: Access::default() };
        assert!(can_view_project(&project, Some(&user)), "Public project should be accessible with any user.");

        let project = Project { visibility: ProjectVisibility::Unlisted, ..project };
        assert!(can_view_project(&project, None), "Unlisted project should be accessible by direct link.");
        assert!(!can_enumerate_project(&project, None), "Unlisted project should not be enumerable anonymously.");
        assert!(
            !can_enumerate_project(&project, Some(&user)),
            "Unlisted project should not be enumerable by unrelated users."
        );

        let assigned_user = User {
            username: "assigned".to_string(),
            role: UserRole::User,
            access: Access {
                projects: AccessScope::Selected(vec!["public_project".to_string()]),
                permissions: [AccessPermission::ProjectRead].into(),
                ..Default::default()
            },
        };
        assert!(
            can_enumerate_project(&project, Some(&assigned_user)),
            "Assigned users can enumerate unlisted projects."
        );

        let project = Project {
            id: "admin_test_project".to_string(),
            display_name: "Admin Test Project".to_string(),
            secret: None,
            visibility: ProjectVisibility::Private,
        };
        let admin_user = User { username: "admin".to_string(), role: UserRole::Admin, access: Access::default() };
        assert!(can_view_project(&project, Some(&admin_user)), "Admin should have access to any project.");
        assert!(admin_user.can_read_entity("site"));
        assert!(!assigned_user.can_read_entity("site"));

        let project = Project {
            id: "private_project".to_string(),
            display_name: "Private Project".to_string(),
            secret: None,
            visibility: ProjectVisibility::Private,
        };
        assert!(!can_view_project(&project, None), "Private project should not be accessible without a user.");
        let internal = Project { visibility: ProjectVisibility::Internal, ..project };
        assert!(!can_view_project(&internal, None));
        assert!(can_view_project(&internal, Some(&user)));
        assert!(can_enumerate_project(&internal, Some(&user)));
    }
}
