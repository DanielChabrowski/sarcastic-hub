use crate::resource::Resource;
use async_trait::async_trait;

#[async_trait]
pub trait ResourceManager {
    fn add_resource(&mut self, resource: Resource);
}
