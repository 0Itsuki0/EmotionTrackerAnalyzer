use serde_json::json;
use anyhow::Result;
use super::tools::{ToDocument, ToolDefinition};


pub fn get_emotion_scores_tool_definition() -> Result<ToolDefinition> {
    let name = "print_emotion_scores";
    let description = "Print emotion score of a given text.";

    let json_schema = json!({
        "type": "object",
        "properties": {
            "fear": {
                "type": "number",
                "description": "Score for fear, ranging from 0.0 to 1.0.",
            },
            "anger": {
                "type": "number",
                "description": "Score for anger, ranging from 0.0 to 1.0.",
            },
            "joy": {
                "type": "number",
                "description": "Score for joy, ranging from 0.0 to 1.0.",
            },
            "sad": {
                "type": "number",
                "description": "Score for sad, ranging from 0.0 to 1.0.",
            },
            "contempt": {
                "type": "number",
                "description": "Score for contempt, ranging from 0.0 to 1.0.",
            },
            "disgust": {
                "type": "number",
                "description": "Score for disgust, ranging from 0.0 to 1.0.",
            },
            "surprise": {
                "type": "number",
                "description": "Score for surprise, ranging from 0.0 to 1.0.",
            },
        },
        "required": ["fear", "anger", "joy", "sad", "contempt", "disgust", "surprise"],
    });

    let schema = json_schema.to_document();
    Ok(ToolDefinition::new(name, description, &schema))
}
