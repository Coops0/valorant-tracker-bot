extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HendrixMmrResponse {
    pub status: i64,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub data: Option<Vec<MmrDatum>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MmrDatum {
    #[serde(rename = "currenttier")]
    pub current_tier: i64,
    #[serde(rename = "currenttierpatched")]
    pub current_tier_patched: String,
    pub images: Images,
    pub ranking_in_tier: i64,
    pub mmr_change_to_last_game: i64,
    pub elo: i64,
    pub date: String,
    pub date_raw: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Images {
    pub small: String,
    pub large: String,
    pub triangle_down: String,
    pub triangle_up: String,
}
