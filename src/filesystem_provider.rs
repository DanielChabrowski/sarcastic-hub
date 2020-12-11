use crate::provider::Provider;
use crate::resource::Resource;
use async_trait::async_trait;
use walkdir::WalkDir;

pub struct FilesystemProvider {
    pub name: String,
    pub paths: Vec<String>,
    pub extensions: Vec<String>,
}

impl FilesystemProvider {
    pub fn new(name: String, paths: Vec<String>, extensions: Vec<String>) -> Self {
        Self {
            name,
            paths,
            extensions,
        }
    }
}

#[async_trait]
impl Provider for FilesystemProvider {
    fn get_name(&self) -> &str {
        &self.name
    }

    async fn fetch(&self) -> Vec<Resource> {
        let mut resources = Vec::new();

        for path in &self.paths {
            for entry in WalkDir::new(path).into_iter() {
                match entry {
                    Ok(entry) => {
                        let p = entry.path();
                        if p.is_file() {
                            let allowed_extension = p.extension().map_or_else(
                                || false,
                                |e| self.extensions.contains(&e.to_str().unwrap().to_string()),
                            );

                            if allowed_extension {
                                resources.push(Resource {
                                    name: p
                                        .strip_prefix(path)
                                        .unwrap()
                                        .to_str()
                                        .unwrap()
                                        .to_string(),
                                })
                            }
                        }
                    }
                    Err(e) => log::error!("walkdir error: {:?}", e),
                }
            }
        }

        resources
    }
}
