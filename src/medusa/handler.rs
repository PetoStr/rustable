use crate::medusa::{AuthRequestData, MedusaAnswer, SharedContext};
use async_trait::async_trait;

#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    async fn handle(&self, context: &SharedContext, auth_data: AuthRequestData) -> MedusaAnswer;
}
