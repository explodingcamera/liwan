use crate::{
    api::SessionUser,
    app::models::{Project, UserRole},
};

pub(crate) const MAX_DATAPOINTS: u32 = 100;

pub(crate) fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

pub(crate) fn is_valid_username(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && name.len() <= 64 && name.len() >= 3
}

pub(crate) fn can_access_project(project: &Project, user: &Option<&SessionUser>) -> bool {
    project.public || user.map_or(false, |u| u.0.role == UserRole::Admin || u.0.projects.contains(&project.id))
}
