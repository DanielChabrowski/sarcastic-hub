use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Resource {
    pub uuid: uuid::Uuid,
    pub name: String,
    pub path: PathBuf,
}
