use std::{
    fmt::{Display, Formatter},
    time::Duration,
};

use anyhow::bail;
use reqwest::get;
use serenity::{model::id::ChannelId, prelude::GatewayIntents, utils::Color, Client};
use tokio::{task, time::sleep};

use crate::hendrix_response::{Datum, HendrixResponse, TeamEnum};

mod hendrix_response;

const PLAYERS: &[PlayerTag] = &[
    PlayerTag("finicky", "8260"),
    PlayerTag("leirbag", "0001"),
    PlayerTag("mvh", "0001"),
    PlayerTag("Chaz", "HEHR"),
];

const URL: &str = "https://api.henrikdev.xyz/valorant/v3/matches/na/";

struct PlayerTag<'a>(&'a str, &'a str);

impl Display for PlayerTag<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}#{}", self.0, self.1))
    }
}

#[tokio::main]
async fn main() {
    let mut last_games: Vec<(&PlayerTag, Option<String>)> =
        PLAYERS.iter().map(|id| (id, None)).collect();

    let mut client = Client::builder(
        "OTYzMjM2NjEwMzA3MTQxNjUw.G8lORi.wUvZlt5uHvRM0ty2UA9XVlCq5in4ic7QuR9qzc",
        GatewayIntents::default(),
    )
    .await
    .unwrap();

    let ctx = client.cache_and_http.clone();

    task::spawn(async move {
        client.start().await.expect("Client failed to start");
    });

    loop {
        // Clone so we can mutate it in the loop
        for (i, (id, last_stored_game)) in last_games.clone().iter().enumerate() {
            let PlayerTag(name, tag) = id;

            let game = match lookup_player(name, tag).await {
                Ok(o) => o,
                Err(e) => {
                    println!("Failed to get player info for {} -> {e}", id);
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
                    println!("Failed to find player in match players ({})!", id);
                    continue;
                }
            };

            let player_stats = &player.stats;
            last_games[i] = (id, Some(last_game_id.clone()));

            if let Some(last_stored_game) = last_stored_game {
                if last_stored_game == &last_game_id {
                    continue;
                }
            } else {
                continue;
            }

            let kd = (player_stats.kills + player_stats.assists) / player_stats.deaths;
            let headshot_percent = format!(
                "{:.0}%",
                (player_stats.head_shots as f64
                    / (player_stats.head_shots + player_stats.body_shots + player_stats.leg_shots)
                        as f64)
                    * 100_f64
            );

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
                field("KD Ratio", kd),
                field("Head Shot Percentage", headshot_percent),
                field("Score", player_stats.score),
                field("Player Rank", &player.current_tier_patched),
                field("Player Level", player.level),
                field("Map", &metadata.map),
            ];

            let message = ChannelId(1010348129771589782).send_message(&ctx.http, |m| {
                m.embed(|e| e
                    .title(format!("{}'s Game on {}", name, metadata.map))
                    .color(Color::BLURPLE)
                    .image(&player.assets.card.wide)
                    .thumbnail(&player.assets.agent.bust)
                    .description(format!(
                        "{name} {} their game on {} with {} kills and {} deaths, and with a KD of **{kd}**, and is now at rank {}",
                        if player_team.has_won { "won" } else { "lost" }, metadata.map, player_stats.kills, player_stats.deaths, player.current_tier_patched
                    ))
                    .fields(fields)
                )
            });

            if let Err(e) = message.await {
                println!("Failed to send message ({}) -> {e}", id);
            }
        }

        sleep(Duration::from_secs(30)).await;
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

fn field<A: ToString, B: ToString>(key: A, value: B) -> (String, String, bool) {
    (key.to_string(), value.to_string(), true)
}
