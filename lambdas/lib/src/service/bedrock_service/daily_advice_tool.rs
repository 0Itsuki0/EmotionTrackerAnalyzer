use serde_json::json;
use anyhow::Result;
use super::tools::{ToDocument, ToolDefinition};


pub fn get_daily_advice_tool_definition() -> Result<ToolDefinition> {
    let name = "print_advice_recommendation";
    let description = "Print advice and song recommendation.";

    let json_schema = json!({
        "type": "object",
        "properties": {
            "advice": {
                "type": "string",
                "description": "The one sentence advice.",
            },
            "song": {
                "type": "string",
                "description": "The name of the song.",
            }
        },
        "required": ["advice", "song"],
    });

    let schema = json_schema.to_document();
    Ok(ToolDefinition::new(name, description, &schema))
}
