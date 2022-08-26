use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

use serenity::{CacheAndHttp, Client, model::id::UserId, prelude::GatewayIntents};
use tokio::{main, task};

use crate::{
    game_tracker::game_tracker_thread,
    hendrix_matches_response::{HendrixMatchesResponse, MatchDatum, Player, TeamEnum},
    hendrix_mmr_response::{HendrixMmrResponse, MmrDatum},
    mmr_tracker::mmr_tracker_thread,
};

mod game_tracker;
mod hendrix_matches_response;
mod hendrix_mmr_response;
mod mmr_tracker;

pub const BASE_URL: &str = "https://api.henrikdev.xyz";
pub const MATCH_URL: &str = "/valorant/v3/matches/na";
pub const MMR_HISTORY_URL: &str = "/valorant/v1/mmr-history/na";

#[main]
async fn main() {
    let mut players = vec![
        PlayerData::new("finicky", "8260", 391061411813523474),
        PlayerData::new("leirbag", "0001", 430013185056178176),
        PlayerData::new("mvh", "0001", 412278960458694666),
        PlayerData::new("Chaz", "HEHR", 408054716723888138),
        PlayerData::new("rvulyobdeifitreC", "0001", 412278960458694666),
        PlayerData::new("jeremyawesome", "NA1", 406956734154932235),
        PlayerData::new("bakon", "8597", 435920046238466049),
        PlayerData::new("CoopyPoopy", "whojo", 788436069003165718),
    ];

    let mut client = Client::builder(
        "OTYzMjM2NjEwMzA3MTQxNjUw.G8lORi.wUvZlt5uHvRM0ty2UA9XVlCq5in4ic7QuR9qzc",
        GatewayIntents::default(),
    )
        .await
        .unwrap();

    let ctx = client.cache_and_http.clone();
    for player in &mut players {
        player.populate_discord_name(&ctx).await;
    }

    task::spawn(game_tracker_thread(
        players.clone(),
        ctx.clone(),
        1010348129771589782,
    ));

    task::spawn(mmr_tracker_thread(players, ctx, 1011839991376248902));

    client.start().await.expect("ERROR: Client failed to start");
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct PlayerData<'a> {
    pub name: &'a str,
    pub tag: &'a str,
    pub discord_id: UserId,

    pub cached_discord_name: Option<String>,
}

impl<'a> PlayerData<'a> {
    fn new<T>(name: &'a str, tag: &'a str, discord_id: T) -> Self
        where
            T: Into<UserId>,
    {
        Self {
            name,
            tag,
            discord_id: discord_id.into(),
            cached_discord_name: None,
        }
    }

    pub async fn populate_discord_name(&mut self, ctx: &Arc<CacheAndHttp>) {
        self.cached_discord_name = self.discord_id.to_user(ctx).await.map(|u| u.name).ok();
    }

    pub fn username(&self) -> String {
        self.cached_discord_name
            .as_ref()
            .map(String::clone)
            .unwrap_or_else(|| self.name.to_string())
    }
}

impl Display for PlayerData<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}#{}", self.name, self.tag))
    }
}
