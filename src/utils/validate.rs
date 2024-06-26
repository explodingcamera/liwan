pub const MAX_DATAPOINTS: u32 = 100;

pub fn is_valid_id(id: &str) -> bool {
    id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
}
