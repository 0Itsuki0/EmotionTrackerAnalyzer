
use std::collections::HashMap;
use aws_smithy_types::Document;
use serde_json::{json, Value};


pub trait ToDocument {
    fn to_document(&self) -> Document;
}

pub trait ToValue {
    fn to_value(&self) -> Value;
}

impl ToDocument for Value {
    fn to_document(&self) -> Document {
        if let Some(string) = self.as_str() {
            return Document::String(string.to_owned());
        }

        if let Some(_) = self.as_null() {
            return Document::Null
        }

        if let Some(bool) = self.as_bool() {
            return Document::Bool(bool)
        }

        if let Some(f64) = self.as_f64() {
            return Document::Number(aws_smithy_types::Number::Float(f64));
        }

        if let Some(array) = self.as_array() {
            let mut doc_array: Vec<Document> = vec![];
            for item in array {
                doc_array.push(item.to_document())
            }
            return Document::Array(doc_array);
        }

        if let Some(object) = self.as_object() {
            let mut doc_map: HashMap<String, Document> = HashMap::new();
            for (key, value) in object.into_iter() {
                doc_map.insert(key.to_owned(), value.to_document());
            };
            return Document::Object(doc_map);
        }

        return Document::Null;
    }
}


impl ToValue for Document {
    fn to_value(&self) -> Value {
        match self {
            Document::Object(map) => {
                let mut value_map: HashMap<String, Value> = HashMap::new();
                for (key, value) in map.into_iter() {
                    value_map.insert(key.to_owned(), value.to_value());
                };
                json!(value_map)
            },
            Document::Array(array) => {
                let mut value_array: Vec<Value> = vec![];
                for item in array {
                    value_array.push(item.to_value())
                }
                return json!(value_array)
            },
            Document::Number(number) =>json!(number.to_f64_lossy()),
            Document::String(str) => json!(str),
            Document::Bool(bool) => json!(bool),
            Document::Null => json!(null),
        }
    }
}


pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub schema: Document
}

impl ToolDefinition {
    pub fn new(name: &str, description: &str, schema: &Document) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            schema: schema.to_owned()
        }
    }
}
