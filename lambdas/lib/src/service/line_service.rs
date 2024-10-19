

use anyhow::{bail, Context, Result};
use reqwest::{header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}, Client,};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{env_keys::{ BOT_OAUTH_TOKEN, RESULT_CHANNEL_ID, SLACK_VERIFICATION_TOKEN}, utilities::get_previous_weekday,};
use super::common_structs::{DailyAdvice, EmotionScores};

pub const EVENT_CALLBACK_TYPE: &str = "event_callback";
pub const MESSAGE_EVENT_TYPE: &str = "message";
const VERIFICATION_TYPE: &str = "url_verification";

const POST_MESSAGE_ENDPOINT: &str = "https://slack.com/api/chat.postMessage";

#[derive(Debug, Clone)]
pub struct LineService {
    client: Client,
    headers: HeaderMap
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct EventChallengeRequest {
    pub challenge: String,
    pub token: String,
    pub r#type: String
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MessageEventRequest {
    pub api_app_id: String,
    pub event_id: String,
    pub event_time: u64,
    pub is_ext_shared_channel: bool,
    pub token: String,
    pub r#type: String,
    pub event: MessageEvent
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MessageEvent {
    pub channel: String,
    pub channel_type: String, // channel, im
    pub r#type: String, // message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>, // none for user message
    pub event_ts: String, // thread_ts
    pub text: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<String>, // none for user message
}


impl LineService {
    pub fn new() -> Self {
        let token: String = std::env::var(BOT_OAUTH_TOKEN).unwrap_or("".to_owned());
        let mut headers = HeaderMap::new();
        let bearer = format!("Bearer {}", token).to_string();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&bearer).unwrap_or(HeaderValue::from_static("")));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json;charset=UTF-8"));

        Self {
            client: Client::new(),
            headers
        }
    }

    pub fn verify_challenge(&self, challenge_request: &EventChallengeRequest) -> Result<bool> {
        let verification_token = std::env::var(SLACK_VERIFICATION_TOKEN)?;
        Ok(verification_token == challenge_request.token && challenge_request.r#type == VERIFICATION_TYPE)
    }

    pub fn verify_message_request(&self, message_request: &MessageEventRequest) -> Result<()> {
        if message_request.r#type != EVENT_CALLBACK_TYPE || message_request.event.r#type != MESSAGE_EVENT_TYPE  {
            bail!("Wrong Event Type");
        }

        if message_request.event.bot_id.is_some() {
            bail!("Bot message.");
        }

        if message_request.event.subtype.is_some() {
            let subtype = message_request.event.subtype.clone().unwrap();
            if subtype.contains("bot") || subtype.contains("channel") || subtype.contains("notification") {
                bail!("Bot/Channel notifications.");
            }
        }
        if message_request.event.text.is_empty() {
            bail!("Empty text.");
        }

        Ok(())
    }

    pub async fn send_daily_thread(&self) -> Result<String> {
        let channel_id = std::env::var(RESULT_CHANNEL_ID)?;

        let date = get_previous_weekday()?;
        let body = json!({
            "channel": channel_id,
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!(":star::star: *{}* :star::star:\nCheck out how you did yesterday and start your day off with AI recommended song!", date)
                    }
                }
            ]
        });

        let response = self.client
            .post(POST_MESSAGE_ENDPOINT)
            .headers(self.headers.clone())
            .body(serde_json::to_string(&body)?)
            .send()
            .await?;

        let body_string = response.text().await?;
        println!("response_body: {}", body_string);
        let body = serde_json::from_str::<Value>(&body_string)?;
        let thread_ts = body.get("ts").context("unable to get thread ts")?.to_string();

        Ok(thread_ts)
    }


    pub async fn send_daily_advice(&self, thread_ts: &str, user_id: &str, advice: &DailyAdvice, max_anger: &(EmotionScores, String), max_contempt: &(EmotionScores, String), max_disgust: &(EmotionScores, String)) -> Result<()> {
        let channel_id = std::env::var(RESULT_CHANNEL_ID)?;
        println!("channel id: {}", channel_id);

        let message_anger = if max_anger.0.anger >= 0.4 {
            format!("*Message with max anger ({})*: {}\n", max_anger.0.anger, max_anger.1) } else { "".to_owned() };
        let message_contempt = if max_contempt.0.contempt >= 0.4 {
            format!("*Message with max contempt ({})*: {}\n", max_contempt.0.contempt, max_contempt.1) } else { "".to_owned() };
        let message_disgust = if max_disgust.0.disgust >= 0.4 {
            format!("*Message with max disgust ({})*: {}\n", max_disgust.0.disgust, max_disgust.1) } else { "".to_owned() };

        let message = message_anger + &message_contempt + &message_disgust + &format!("*Advice*: {}", advice.advice) + "\n" + &format!("*Song Recommendation*: {}", advice.song);

        let body = json!({
            "channel": channel_id,
            "thread_ts": thread_ts,
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!(":heart: <@{}> :heart:\n{}", user_id, message)
                    }
                }
            ]
        });

        let response = self.client
            .post(POST_MESSAGE_ENDPOINT)
            .headers(self.headers.clone())
            .body(serde_json::to_string(&body)?)
            .send()
            .await?;

        let body_string = response.text().await?;
        println!("response_body: {}", body_string);

        Ok(())
    }


    pub async fn send_immediate_warning(&self, channel_id: &str, thread_ts: &str, user_id: &str, message: &str) -> Result<()>{
        let body = json!({
            "channel": channel_id,
            "thread_ts": thread_ts,
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!(":warning:<@{}>:warning:\n{}", user_id, message)
                    }
                }
            ]
        });

        let response = self.client
            .post(POST_MESSAGE_ENDPOINT)
            .headers(self.headers.clone())
            .body(serde_json::to_string(&body)?)
            .send()
            .await?;

        let body_string = response.text().await?;
        println!("response_body: {}", body_string);

        Ok(())
    }
}