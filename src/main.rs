use std::{
    fmt::{Display, Formatter},
    time::Duration,
};

use anyhow::bail;
use reqwest::get;
use serenity::{
    model::id::ChannelId,
    prelude::GatewayIntents,
    utils::Color,
    Client,
    model::prelude::UserId,
};
use tokio::{task, time::sleep};

use crate::hendrix_response::{Datum, HendrixResponse, Player, TeamEnum};

mod hendrix_response;

const URL: &str = "https://api.henrikdev.xyz/valorant/v3/matches/na/";

struct PlayerData<'a> {
    name: &'a str,
    tag: &'a str,
    discord_id: UserId,
}

impl<'a> PlayerData<'a> {
    fn new<T>(name: &'a str, tag: &'a str, discord_id: T) -> Self
        where T: Into<UserId> {
        Self {
            name,
            tag,
            discord_id: discord_id.into(),
        }
    }
}

impl Display for PlayerData<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}#{}", self.name, self.tag))
    }
}

#[tokio::main]
async fn main() {
    let players = vec![
        PlayerData::new("finicky", "8260", 391061411813523474),
        PlayerData::new("leirbag", "0001", 430013185056178176),
        PlayerData::new("mvh", "0001", 412278960458694666),
        PlayerData::new("Chaz", "HEHR", 408054716723888138),
        PlayerData::new("rvulyobdeifitreC", "0001", 412278960458694666),
        PlayerData::new("jeremyawesome", "NA1", 406956734154932235),
    ];

    let mut last_games = players
        .iter()
        .map(|p| (p, None))
        .collect::<Vec<(&PlayerData, Option<String>)>>();

    let mut client = Client::builder(
        "OTYzMjM2NjEwMzA3MTQxNjUw.G8lORi.wUvZlt5uHvRM0ty2UA9XVlCq5in4ic7QuR9qzc",
        GatewayIntents::default(),
    )
        .await
        .unwrap();

    let ctx = client.cache_and_http.clone();

    task::spawn(async move {
        client.start().await.expect("ISSUE: Client failed to start");
    });

    loop {
        // Clone so we can mutate it in the loop
        for (i, (id, last_stored_game)) in last_games.clone().iter().enumerate() {
            let PlayerData { name, tag, discord_id } = id;

            let game = match lookup_player(name, tag).await {
                Ok(o) => o,
                Err(e) => {
                    println!("ERROR: Failed to get player info for {id} -> {e}");
                    continue;
                }
            };

            let metadata = &game.metadata;
            let last_game_id = metadata.match_id.clone();

            let player = match game
                .players
                .all_players
                .iter()
                .find(|p| p.name.as_str() == *name && p.tag.as_str() == *tag)
            {
                Some(o) => o,
                None => {
                    println!("ERROR: Failed to find player in match players ({id})!");
                    continue;
                }
            };

            let player_stats = &player.stats;
            last_games[i] = (id, Some(last_game_id.clone()));

            if let Some(last_stored_game) = last_stored_game {
                if last_stored_game == &last_game_id {
                    println!("INFO: Last stored game is same as newest for {id}");
                    continue;
                }
            } else {
                println!("INFO: No game stored, so no need to send match message for {id}");
                continue;
            }

            let kd = format!(
                "{:.2}",
                calculate_kd(player)
            );

            let mut kd_ranking = game
                .players
                .all_players
                .iter()
                .map(|p| (p, calculate_kd(p)))
                .collect::<Vec<(&Player, f64)>>();
            kd_ranking
                .sort_by(|(_, akb), (_, bkb)| bkb.partial_cmp(akb).unwrap());

            let position = kd_ranking
                .iter()
                .enumerate()
                .find(|(_, (p, _))| p.puuid == player.puuid)
                .unwrap() // Should NEVER fail
                .0 + 1; // It's an index so add one

            // this is cancerous but not really a better way to do this that doesn't require just moving it into the other file
            let player_team = if player.team == TeamEnum::Red {
                &game.teams.red
            } else {
                &game.teams.blue
            };

            let fields = vec![
                field("Rounds", metadata.rounds_played),
                field(
                    "Player Team Rounds Won / Lost",
                    format!("{} / {}", player_team.rounds_won, player_team.rounds_lost),
                ),
                field(
                    "Game Length",
                    format!("{}min", metadata.game_length / 60000),
                ),
                field("Agent", &player.character),
                field("Kills", player_stats.kills),
                field("Assists", player_stats.assists),
                field("Deaths", player_stats.deaths),
                field("KD Ratio", &kd),
                field("Leaderboard Position", position),
                field("Head Shot Percentage", format!("{}%", calculate_headshot_percentage(player) as i64)),
                field("Score", player_stats.score),
                field("Player Rank", &player.current_tier_patched),
                field("Map", &metadata.map),
            ];

            let username = match discord_id.to_user(&ctx.http).await {
                Ok(u) => u.name,
                Err(_) => name.to_string(),
            };

            let message = ChannelId(1010348129771589782).send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title(format!("{}'s Game on {}", username, metadata.map))
                        .color(if player_team.has_won {
                            Color::DARK_GREEN
                        } else {
                            Color::DARK_RED
                        })
                        .image(&player.assets.card.wide)
                        .thumbnail(&player.assets.agent.bust)
                        .description(format!(
                            "{name} **{}** their game on {} with a KD of {kd}, and is now at rank {}",
                            if player_team.has_won { "won" } else { "lost" },
                            metadata.map,
                            player.current_tier_patched
                        ))
                        .fields(fields)
                })
            }).await;

            match message {
                Ok(_) => println!("SUCCESS: Sent new match message for {id}"),
                Err(e) => println!("ERROR: Failed to send message ({id}) -> {e}"),
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn lookup_player(name: &str, tag: &str) -> anyhow::Result<Datum> {
    let response = get(format!("{URL}{name}/{tag}?filter=competitive")).await?;
    let parsed = response.json::<HendrixResponse>().await?;

    if parsed.status != 200 {
        bail!("got status of {} instead of 200.", parsed.status);
    }

    match parsed.data {
        Some(mut d) if !d.is_empty() => Ok(d.remove(0)),
        _ => bail!("no matches found"),
    }
}

fn calculate_kd(player: &Player) -> f64 {
    player.stats.kills as f64 / player.stats.deaths as f64
}

fn calculate_headshot_percentage(player: &Player) -> f64 {
    let all_shots = (player.stats.head_shots + player.stats.body_shots + player.stats.leg_shots) as f64;

    (player.stats.head_shots as f64 / all_shots) * 100_f64
}

#[inline]
fn field<A: ToString, B: ToString>(key: A, value: B) -> (String, String, bool) {
    (key.to_string(), value.to_string(), true)
}
