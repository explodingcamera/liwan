pub const MAX_DATAPOINTS: u32 = 100;

pub fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}

pub fn is_valid_username(name: &str) -> bool {
    name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') && name.len() <= 64 && name.len() >= 3
}
