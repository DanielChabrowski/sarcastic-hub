use crate::provider::Provider;
use crate::resource::Resource;
use async_trait::async_trait;

pub struct FilesystemProvider {
    pub name: String,
    pub paths: Vec<String>,
}

impl FilesystemProvider {
    pub fn new(name: String, paths: Vec<String>) -> Self {
        Self { name, paths }
    }
}

#[async_trait]
impl Provider for FilesystemProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn search(&self, _query: String) -> Vec<Resource> {
        vec![Resource {
            name: "example resource".into(),
        }]
    }
}
