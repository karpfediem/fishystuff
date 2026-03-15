use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{ChannelId, Http};

#[async_trait::async_trait]
pub trait Purger: Send + Sync {
    async fn bulk_delete(
        &self,
        http: &Http,
        channel: ChannelId,
        ids: &[serenity::MessageId],
    ) -> serenity::Result<()>;

    async fn single_delete(
        &self,
        http: &Http,
        channel: ChannelId,
        id: serenity::MessageId,
    ) -> serenity::Result<()>;
}

/// Default purger backed by Serenity HTTP.
pub struct SerenityPurger;

impl SerenityPurger {
    pub fn new() -> Self {
        Self {}
    }
}
#[async_trait::async_trait]
impl Purger for SerenityPurger {
    async fn bulk_delete(
        &self,
        http: &Http,
        channel: ChannelId,
        ids: &[serenity::MessageId],
    ) -> serenity::Result<()> {
        channel.delete_messages(http, ids.to_vec()).await
    }

    async fn single_delete(
        &self,
        http: &Http,
        channel: ChannelId,
        id: serenity::MessageId,
    ) -> serenity::Result<()> {
        channel.delete_message(http, id).await
    }
}
