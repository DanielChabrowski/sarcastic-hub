use crate::provider::{Provider, ResourceProviderInterface};
use crate::resource::Resource;
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender as Sender;
use walkdir::WalkDir;

pub struct FilesystemProvider {
    pub name: String,
    pub paths: Vec<String>,
    pub extensions: Vec<String>,
}

impl FilesystemProvider {
    pub fn new(
        name: String,
        paths: Vec<String>,
        extensions: Vec<String>,
        resource_sender: Sender<ResourceProviderInterface>,
    ) -> Self {
        let zelf = Self {
            name,
            paths: paths.clone(),
            extensions: extensions.clone(),
        };

        tokio::spawn(async move {
            let res = fetch(paths, extensions);
            resource_sender
                .send(ResourceProviderInterface::Add(res))
                .unwrap();
        });

        zelf
    }
}

fn fetch(paths: Vec<String>, extensions: Vec<String>) -> Vec<Resource> {
    let mut resources = Vec::new();

    for path in &paths {
        for entry in WalkDir::new(path)
            .sort_by_key(|a| a.file_name().to_owned())
            .into_iter()
        {
            match entry {
                Ok(entry) => {
                    let p = entry.path();
                    if p.is_file() {
                        let allowed_extension = p.extension().map_or_else(
                            || false,
                            |e| extensions.contains(&e.to_str().unwrap().to_string()),
                        );

                        if allowed_extension {
                            resources.push(Resource {
                                uuid: uuid::Uuid::new_v4(),
                                name: p.strip_prefix(path).unwrap().to_str().unwrap().to_string(),
                                path: p.to_path_buf(),
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

#[async_trait]
impl Provider for FilesystemProvider {
    fn get_name(&self) -> &str {
        &self.name
    }
}
