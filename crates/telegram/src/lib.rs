use anyhow::Result;
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ParseMode;
use teloxide::{Bot, prelude::Requester, types::ChatId};

pub struct TelegramClient {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramClient {
    pub fn new(bot_token: &str, chat_id: i64) -> Self {
        Self {
            bot: Bot::new(bot_token),
            chat_id: ChatId(chat_id),
        }
    }

    #[inline]
    pub async fn send_message(&self, message: String) -> Result<()> {
        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::Html)
            .await?;
        Ok(())
    }
}
