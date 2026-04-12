use anyhow::Result;

pub struct TelegramAdapter;

impl TelegramAdapter {
    pub fn new(_token: &str) -> Self {
        println!("Telegram adapter initialized");
        Self
    }

    pub async fn send_message(&self, _chat_id: i64, _text: &str) -> Result<()> {
        Ok(())
    }

    pub async fn handle_update(&self, _update: serde_json::Value) -> Result<Option<super::super::session::GatewayMessage>> {
        Ok(None)
    }
}
