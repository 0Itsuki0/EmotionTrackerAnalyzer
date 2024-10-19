
pub mod tools;
pub mod emotion_scores_tool;
pub mod daily_advice_tool;

use core::str;
use std::env;
use anyhow::{bail, Context, Result};
use aws_sdk_bedrockruntime::types::{SpecificToolChoice, ToolChoice};
use aws_sdk_bedrockruntime::Client;
use aws_sdk_bedrockruntime::types::{ContentBlock, Message, SystemContentBlock, Tool, ToolConfiguration, ToolInputSchema, ToolSpecification, ConversationRole::User};
use aws_sdk_bedrockruntime::operation::converse::ConverseOutput;
use daily_advice_tool::get_daily_advice_tool_definition;
use emotion_scores_tool::get_emotion_scores_tool_definition;

use tools::ToValue;
use crate::env_keys::CHAT_MODEL;
use super::common_structs::{DailyAdvice, EmotionScores};


#[derive(Debug, Clone)]
pub struct BedrockService {
    client: Client,
    chat_model_id: String,
}

impl BedrockService {
    pub fn new(client: &aws_sdk_bedrockruntime::Client) -> Self {
        Self {
            client: client.to_owned(),
            chat_model_id: env::var(CHAT_MODEL).unwrap_or("".to_owned())
        }
    }

    pub async fn get_emotion_scroe(&self, text: &str) -> Result<EmotionScores> {

        let tool_definition = get_emotion_scores_tool_definition()?;
        let emotion_scores_tool = Tool::ToolSpec(
            ToolSpecification::builder()
                .name(&tool_definition.name)
                .description(&tool_definition.description)
                .input_schema(ToolInputSchema::Json(tool_definition.schema))
                .build()?
        );

        let tool_config = ToolConfiguration::builder()
            .set_tools(Some(vec![emotion_scores_tool]))
            .tool_choice(ToolChoice::Tool(SpecificToolChoice::builder().name(&tool_definition.name).build()?))
            .build()?;

        let system_prompt = format!("
            You will be acting as an AI Empath.
            Your are an expert at reading emotions within text messages and chats.
            The text given will be a message sent to a Slack Channel of a company.
            The target text will be surrounded by <text></text>.
            You have to use using {} to print out the score for each emotion.
        ", tool_definition.name);

        let message = Message::builder()
            .role(User)
            .content(ContentBlock::Text(format!("<text>{}</text>", text)))
            .build()?;

        let response = self.send(&system_prompt, vec![message], Some(tool_config)).await?;

        println!("response: {:?}", response);
        self.process_emotion_score_output(response, &tool_definition.name)
    }


    fn process_emotion_score_output(&self, response: ConverseOutput, tool_name: &str) -> Result<EmotionScores> {
        let output = response.output.context("Error getting output")?;
        let message = match output.as_message() {
            Ok(message) => message.to_owned(),
            Err(output) => {
                bail!("Converse output is not message: {:?}", output)
            },
        };

        let contents = message.content;
        let mut scores: Option<EmotionScores> = None;

        for content in contents {
            if !content.is_tool_use() {
                continue;
            }
            let tool_use  = match content.as_tool_use() {
                Ok(tool_use) => tool_use,
                Err(block) => {
                    println!("Block: {:?} is not tool use.", block);
                    continue;
                },
            };

            if tool_use.name() != tool_name {
                continue;
            }
            let input = tool_use.input().to_value();
            match serde_json::from_value(input) {
                Ok(s) => {
                    scores = Some(s);
                    break;
                },
                Err(error) => {
                    println!("error getting scores from tool input: {}.", error);
                    continue;
                },
            };
        }

        println!("tool use. name: {}, input: {:?}", tool_name, scores);

        if scores.is_none() {
            bail!("Error getting emotion scores")
        }

        Ok(scores.unwrap())
    }


    pub async fn get_daily_advice(&self, emotion_scores: &Vec<EmotionScores>) -> Result<DailyAdvice> {

        let tool_definition = get_daily_advice_tool_definition()?;
        let daily_advice_tool = Tool::ToolSpec(
            ToolSpecification::builder()
                .name(&tool_definition.name)
                .description(&tool_definition.description)
                .input_schema(ToolInputSchema::Json(tool_definition.schema))
                .build()?
        );

        let tool_config = ToolConfiguration::builder()
            .set_tools(Some(vec![daily_advice_tool]))
            .tool_choice(ToolChoice::Tool(SpecificToolChoice::builder().name(&tool_definition.name).build()?))
            .build()?;

        let system_prompt = format!("
            You are a mental health professional.
            You give advices to employees based on the emotion scores evaluated for the text messages they sent to Slack through out the day.
            The emotion scores for a single emplyee in a single day will be given in the following format.

            <scores>
            {{anger: 0.6, contempt: 0.0, disgust: 0.0, fear: 0.1, joy: 0.6, surprise: 0.0, sad: 0.1}}
            {{anger: 0.8, contempt: 0.0, disgust: 0.0, fear: 0.1, joy: 0.0, surprise: 0.0, sad: 0.1}}
            ...
            <scores>

            Each line represents a set of emotion scores for a single text message.
            Lines are in the order of when the text message is sent. Earliest comes first.
            Each score is evaluated in the range of 0.0 to 1.0.

            You are job is to
            - Give a one sentence advice
            - Recommend a song to listen.

            You have to use using {} to print out the advice and the recommended song.
        ", tool_definition.name);


        let score_string_vec = emotion_scores.to_owned()
            .into_iter()
            .map(|s| serde_json::to_string(&s).unwrap_or("".to_owned()))
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>();

        let message_text = format!("<scores>\n{}\n<scores>", score_string_vec.join("\n"));

        println!("message sent: {:?}", message_text);

        let message = Message::builder()
            .role(User)
            .content(ContentBlock::Text(message_text))
            .build()?;

        let response = self.send(&system_prompt, vec![message], Some(tool_config)).await?;

        println!("response: {:?}", response);
        self.process_daily_advice_output(response, &tool_definition.name)
    }


    fn process_daily_advice_output(&self, response: ConverseOutput, tool_name: &str) -> Result<DailyAdvice> {
        let output = response.output.context("Error getting output")?;
        let message = match output.as_message() {
            Ok(message) => message.to_owned(),
            Err(output) => {
                bail!("Converse output is not message: {:?}", output)
            },
        };

        let contents = message.content;
        let mut advice: Option<DailyAdvice> = None;

        for content in contents {
            if !content.is_tool_use() {
                continue;
            }
            let tool_use  = match content.as_tool_use() {
                Ok(tool_use) => tool_use,
                Err(block) => {
                    println!("Block: {:?} is not tool use.", block);
                    continue;
                },
            };

            if tool_use.name() != tool_name {
                continue;
            }
            let input = tool_use.input().to_value();
            match serde_json::from_value(input) {
                Ok(a) => {
                    advice = Some(a);
                    break;
                },
                Err(error) => {
                    println!("error getting advices from tool input: {}.", error);
                    continue;
                },
            };
        }

        println!("tool use. name: {}, input: {:?}", tool_name, advice);

        if advice.is_none() {
            bail!("Error getting daily advice")
        }

        Ok(advice.unwrap())
    }


    async fn send(&self, system_prompt: &str, messages: Vec<Message>, tool_config: Option<ToolConfiguration>) -> Result<ConverseOutput> {
        let builder = self.client
            .converse()
            .model_id(&self.chat_model_id)
            .system(SystemContentBlock::Text(system_prompt.to_owned()))
            .set_messages(Some(messages))
            .set_tool_config(tool_config);

        let response = builder
            .send()
            .await?;
        Ok(response)
    }

}
