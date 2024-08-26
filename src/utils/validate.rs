use crate::{
    app::models::{Project, UserRole},
    web::SessionUser,
};

pub const MAX_DATAPOINTS: u32 = 100;

pub fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

pub fn is_valid_username(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && name.len() <= 64 && name.len() >= 3
}

pub fn can_access_project(project: &Project, user: Option<&SessionUser>) -> bool {
    project.public || user.map_or(false, |u| u.0.role == UserRole::Admin || u.0.projects.contains(&project.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::models::{Project, User};

    #[test]
    fn test_can_access_project() {
        let project = Project {
            id: "public_project".to_string(),
            display_name: "Public Project".to_string(),
            secret: None,
            public: true,
        };
        assert!(can_access_project(&project, None), "Public project should be accessible without a user.");

        let user = SessionUser(User {
            username: "test".to_string(),
            role: UserRole::User,
            projects: vec!["other".to_string()],
        });
        assert!(can_access_project(&project, Some(&user)), "Public project should be accessible with any user.");

        let project = Project {
            id: "admin_test_project".to_string(),
            display_name: "Admin Test Project".to_string(),
            secret: None,
            public: false,
        };
        let admin_user = SessionUser(User { username: "admin".to_string(), role: UserRole::Admin, projects: vec![] });
        assert!(can_access_project(&project, Some(&admin_user)), "Admin should have access to any project.");

        let project = Project {
            id: "private_project".to_string(),
            display_name: "Private Project".to_string(),
            secret: None,
            public: false,
        };
        assert!(!can_access_project(&project, None), "Private project should not be accessible without a user.");
    }
}
