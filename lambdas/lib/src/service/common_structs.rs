
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmotionScores {
    pub anger: f64,
    pub contempt: f64,
    pub disgust: f64,
    pub fear: f64,
    pub joy: f64,
    pub surprise: f64,
    pub sad: f64,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyAdvice {
    pub advice: String,
    pub song: String,
}
