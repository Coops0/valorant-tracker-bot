extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct HendrixResponse {
    pub status: i64,
    pub data: Option<Vec<Datum>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Datum {
    pub metadata: Metadata,
    pub players: Players,
    pub teams: Teams,
    pub rounds: Vec<Round>,
    pub kills: Vec<Kill>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Kill {
    pub kill_time_in_round: i64,
    pub kill_time_in_match: i64,
    pub round: Option<i64>,
    pub killer_puuid: String,
    pub killer_display_name: String,
    pub killer_team: TeamEnum,
    pub victim_puuid: String,
    pub victim_display_name: String,
    pub victim_team: TeamEnum,
    pub victim_death_location: Location,
    pub damage_weapon_id: String,
    pub damage_weapon_name: Option<String>,
    pub damage_weapon_assets: DamageWeaponAssetsClass,
    pub secondary_fire_mode: bool,
    pub player_locations_on_kill: Vec<PlayerLocationsOn>,
    pub assistants: Vec<Assistant>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Assistant {
    pub assistant_puuid: String,
    pub assistant_display_name: String,
    pub assistant_team: TeamEnum,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DamageWeaponAssetsClass {
    pub display_icon: Option<String>,
    pub killfeed_icon: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerLocationsOn {
    pub player_puuid: String,
    pub player_display_name: String,
    pub player_team: TeamEnum,
    pub location: Location,
    pub view_radians: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Location {
    pub x: i64,
    pub y: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub map: String,
    pub game_version: String,
    pub game_length: i64,
    pub game_start: i64,
    pub game_start_patched: String,
    pub rounds_played: i64,
    pub mode: String,
    pub queue: String,
    pub season_id: String,
    pub platform: String,
    #[serde(rename = "matchid")]
    pub match_id: String,
    pub region: String,
    pub cluster: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Players {
    pub all_players: Vec<Player>,
    pub red: Vec<Player>,
    pub blue: Vec<Player>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
    pub puuid: String,
    pub name: String,
    pub tag: String,
    pub team: TeamEnum,
    pub level: i64,
    pub character: String,
    #[serde(rename = "currenttier")]
    pub current_tier: i64,
    #[serde(rename = "currenttier_patched")]
    pub current_tier_patched: String,
    pub player_card: String,
    pub player_title: String,
    pub party_id: Option<String>,
    pub session_playtime: SessionPlaytime,
    pub behavior: Behavior,
    pub platform: PlatformClass,
    pub assets: AllPlayerAssets,
    pub stats: Stats,
    pub economy: AllPlayerEconomy,
    pub damage_made: i64,
    pub damage_received: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AllPlayerAssets {
    pub card: Card,
    pub agent: Agent,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Agent {
    pub small: String,
    pub bust: String,
    pub full: String,
    #[serde(rename = "killfeed")]
    pub kill_feed: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Card {
    pub small: String,
    pub large: String,
    pub wide: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Behavior {
    pub afk_rounds: f64,
    pub friendly_fire: FriendlyFire,
    pub rounds_in_spawn: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendlyFire {
    pub incoming: i64,
    pub outgoing: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AllPlayerEconomy {
    pub spent: LoadoutValue,
    pub loadout_value: LoadoutValue,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LoadoutValue {
    pub overall: i64,
    pub average: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlatformClass {
    #[serde(rename = "type")]
    pub platform_type: String,
    pub os: Os,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Os {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionPlaytime {
    pub minutes: Option<i64>,
    pub seconds: Option<i64>,
    pub milliseconds: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stats {
    pub score: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    #[serde(rename = "bodyshots")]
    pub body_shots: i64,
    #[serde(rename = "headshots")]
    pub head_shots: i64,
    #[serde(rename = "legshots")]
    pub leg_shots: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Round {
    pub winning_team: TeamEnum,
    pub end_type: String,
    pub bomb_planted: bool,
    pub bomb_defused: bool,
    pub plant_events: PlantEvents,
    pub defuse_events: DefuseEvents,
    pub player_stats: Vec<PlayerStat>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DefuseEvents {
    pub defuse_location: Option<Location>,
    pub defused_by: Option<EdBy>,
    pub defuse_time_in_round: Option<i64>,
    pub player_locations_on_defuse: Option<Vec<PlayerLocationsOn>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EdBy {
    pub puuid: String,
    pub display_name: String,
    pub team: TeamEnum,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlantEvents {
    pub plant_location: Option<Location>,
    pub planted_by: Option<EdBy>,
    pub plant_site: Option<PlantSite>,
    pub plant_time_in_round: Option<i64>,
    pub player_locations_on_plant: Option<Vec<PlayerLocationsOn>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerStat {
    pub player_puuid: String,
    pub player_display_name: String,
    pub player_team: TeamEnum,
    pub damage_events: Vec<DamageEvent>,
    pub damage: i64,
    #[serde(rename = "bodyshots")]
    pub body_shots: i64,
    #[serde(rename = "headshots")]
    pub head_shots: i64,
    #[serde(rename = "legshots")]
    pub leg_shots: i64,
    pub kill_events: Vec<Kill>,
    pub kills: i64,
    pub score: i64,
    pub economy: PlayerStatEconomy,
    pub was_afk: bool,
    pub was_penalized: bool,
    pub stayed_in_spawn: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DamageEvent {
    pub receiver_puuid: String,
    pub receiver_display_name: String,
    pub receiver_team: TeamEnum,
    pub damage: i64,
    #[serde(rename = "bodyshots")]
    pub body_shots: i64,
    #[serde(rename = "headshots")]
    pub head_shots: i64,
    #[serde(rename = "legshots")]
    pub leg_shots: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerStatEconomy {
    pub loadout_value: i64,
    pub weapon: Weapon,
    pub armor: Armor,
    pub remaining: i64,
    pub spent: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Armor {
    pub id: Option<String>,
    pub name: Option<ArmorName>,
    pub assets: ArmorAssets,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArmorAssets {
    pub display_icon: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Weapon {
    pub id: Option<String>,
    pub name: Option<String>,
    pub assets: Option<DamageWeaponAssetsClass>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Teams {
    pub red: Team,
    pub blue: Team,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Team {
    pub has_won: bool,
    pub rounds_won: i64,
    pub rounds_lost: i64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum TeamEnum {
    Blue,
    Red,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PlantSite {
    A,
    B,
    C,
    D,
    E,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ArmorName {
    #[serde(rename = "Heavy Shields")]
    HeavyShields,
    #[serde(rename = "Light Shields")]
    LightShields,
}
