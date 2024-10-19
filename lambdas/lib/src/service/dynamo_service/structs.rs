use serde::{Deserialize, Serialize};

use crate::{service::{common_structs::EmotionScores, line_service::MessageEventRequest}, utilities::get_date_month};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmotionTableEntry {
    pub event_id: String,
    pub user_id: String,
    pub timestamp: u64,
    pub date: String,
    pub month: String,

    pub channel_id: String,
    pub channel_type: String, // channel, im
    pub text: String,

    #[serde(flatten)]
    pub scores: EmotionScores
}

impl EmotionTableEntry {
    pub fn new(message_request: &MessageEventRequest, scores: &EmotionScores) -> anyhow::Result<Self> {
        let message_event = message_request.to_owned().event;
        let (date, month) = get_date_month(message_request.event_time)?;
        Ok(
            Self {
                event_id: message_request.event_id.to_owned(),
                user_id: message_event.user,
                timestamp: message_request.event_time,
                date,
                month,
                channel_id: message_event.channel,
                channel_type: message_event.channel_type,
                text: message_event.text,
                scores: scores.to_owned()
            }
        )
    }
}