use anyhow::Result;

pub struct DiscordAdapter;

impl DiscordAdapter {
    pub fn new(_token: &str) -> Self {
        Self
    }

    pub async fn send_message(&self, _channel_id: &str, _text: &str) -> Result<()> {
        Ok(())
    }
}
