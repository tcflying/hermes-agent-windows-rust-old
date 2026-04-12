use anyhow::Result;

pub struct SlackAdapter;

impl SlackAdapter {
    pub fn new(_token: &str) -> Self {
        Self
    }

    pub async fn send_message(&self, _channel: &str, _text: &str) -> Result<()> {
        Ok(())
    }
}
