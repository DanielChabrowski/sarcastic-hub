use crate::resource::Resource;
use async_trait::async_trait;

#[async_trait]
pub trait Provider {
    fn get_name(&self) -> &str;
}

pub enum ResourceProviderInterface {
    Add(Vec<Resource>),
    // Remove
    // Modify
}

impl std::fmt::Debug for ResourceProviderInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceProviderInterface")
    }
}
