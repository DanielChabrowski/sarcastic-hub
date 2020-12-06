use crate::resource::Resource;
use async_trait::async_trait;

#[async_trait]
pub trait Provider {
    fn get_name(&self) -> &str;
    async fn search(&self, query: String) -> Vec<Resource>;
}
