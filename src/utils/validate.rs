use crate::app::models::{Project, User, UserRole};
pub const MAX_DATAPOINTS: u32 = 2000;

pub fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

pub fn is_valid_username(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && name.len() <= 64 && name.len() >= 3
}

pub fn can_view_project(project: &Project, user: Option<&User>) -> bool {
    project.public || user.is_some_and(|u| u.role == UserRole::Admin || u.projects.contains(&project.id))
}

pub fn can_enumerate_project(project: &Project, user: Option<&User>) -> bool {
    can_view_project(project, user)
        && (!project.unlisted || user.is_some_and(|u| u.role == UserRole::Admin || u.projects.contains(&project.id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{Project, User};

    #[test]
    fn test_project_visibility() {
        let project = Project {
            id: "public_project".to_string(),
            display_name: "Public Project".to_string(),
            secret: None,
            public: true,
            unlisted: false,
        };
        assert!(can_view_project(&project, None), "Public project should be accessible without a user.");
        assert!(can_enumerate_project(&project, None), "Public listed project should be enumerable.");

        let user = User { username: "test".to_string(), role: UserRole::User, projects: vec!["other".to_string()] };
        assert!(can_view_project(&project, Some(&user)), "Public project should be accessible with any user.");

        let project = Project { unlisted: true, ..project };
        assert!(can_view_project(&project, None), "Unlisted project should be accessible by direct link.");
        assert!(!can_enumerate_project(&project, None), "Unlisted project should not be enumerable anonymously.");
        assert!(
            !can_enumerate_project(&project, Some(&user)),
            "Unlisted project should not be enumerable by unrelated users."
        );

        let assigned_user = User {
            username: "assigned".to_string(),
            role: UserRole::User,
            projects: vec!["public_project".to_string()],
        };
        assert!(
            can_enumerate_project(&project, Some(&assigned_user)),
            "Assigned users can enumerate unlisted projects."
        );

        let project = Project {
            id: "admin_test_project".to_string(),
            display_name: "Admin Test Project".to_string(),
            secret: None,
            public: false,
            unlisted: false,
        };
        let admin_user = User { username: "admin".to_string(), role: UserRole::Admin, projects: vec![] };
        assert!(can_view_project(&project, Some(&admin_user)), "Admin should have access to any project.");

        let project = Project {
            id: "private_project".to_string(),
            display_name: "Private Project".to_string(),
            secret: None,
            public: false,
            unlisted: false,
        };
        assert!(!can_view_project(&project, None), "Private project should not be accessible without a user.");
    }
}
