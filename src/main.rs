use std::{collections::HashMap, time::Duration};

use anyhow::bail;
use reqwest::get;
use serenity::{model::id::ChannelId, prelude::GatewayIntents, utils::Color, Client};
use tokio::{task, time::sleep};

use crate::hendrix_response::{Datum, HendrixResponse, TeamEnum};

mod hendrix_response;

const PLAYERS: &[(&str, &str)] = &[
    ("finicky", "8260"),
    // ("leirbag", "0001")
];
const URL: &str = "https://api.henrikdev.xyz/valorant/v3/matches/na/";

#[tokio::main]
async fn main() {
    let mut last_games: HashMap<&(&str, &str), Option<String>> = HashMap::new();
    for id in PLAYERS {
        last_games.insert(id, None);
    }

    let mut client = Client::builder(
        "ODg2MjQ4MjE5NTI3NDk5ODE3.YTy0-Q.K2RPLtA5SndZaDUEl1S_Pc35PzM",
        GatewayIntents::empty(),
    )
    .await
    .unwrap();

    let ctx = client.cache_and_http.clone();

    task::spawn(async move {
        client.start().await.expect("Client failed to start");
    });

    loop {
        // Clone so we can mutate it in the loop
        for (id, last_game) in last_games.clone() {
            let (name, tag) = id;
            let game = match lookup_player(name, tag).await {
                Ok(o) => o,
                Err(e) => {
                    println!("Failed to get player info for {:?} -> {e}", id);
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
                    println!("Failed to find player in match players ({:?})!", id);
                    continue;
                }
            };

            let player_stats = &player.stats;
            last_games.insert(id, Some(last_game_id.clone()));

            if let Some(last_game) = last_game {
                if last_game == last_game_id {
                    continue;
                }
            } else {
                // continue;
            }

            // let fields = [
            //     ("Map", &metadata.map),
            //     ("Game Length", metadata.game_length),
            //     ("Rounds", metadata.rounds_played),
            //     (""),
            // ];

            let kd = (player_stats.kills + player_stats.assists) / player_stats.deaths;
            let headshot_percent = format!(
                "{}%",
                (player_stats.head_shots
                    / (player_stats.head_shots + player_stats.body_shots + player_stats.leg_shots))
                    * 100
            );

            // this is cancerous but not really a better way to do this that doesn't require just moving it into the other file
            let player_team = if player.team == TeamEnum::Red {
                &game.teams.red
            } else {
                &game.teams.blue
            };

            let message = ChannelId(1010348129771589782).send_message(&ctx.http, |m| {
                m.embed(|e| e
                    .title(format!("{}'s Game on {}", name, metadata.map))
                    .color(Color::BLURPLE)
                    .image(&player.assets.card.large)
                    .thumbnail(&player.assets.agent.bust)
                    .description(format!(
                        "{name} {} their game on {} with {} kills and {} deaths, and a KD of **{kd}**, and is now at rank {}",
                        if player_team.has_won { "won" } else { "lost" }, metadata.map, player_stats.kills, player_stats.deaths, player.current_tier_patched
                    ))
                    .field("Map", &metadata.map, true)
                    .field("Kills", player_stats.kills, true)
                    .field("Deaths", player_stats.deaths, true)
                    .field("KD Ratio", kd, true)
                    .field("Rounds", metadata.rounds_played, true)
                    .field("Game Length (minutes)", metadata.game_length / 60, true)
                    .field("Head Shot Percentage", headshot_percent, true)
                    .field("Score", player_stats.score, true)
                    .field("Agent", &player.character, true)
                    .field("Player Rank", &player.current_tier_patched, true)
                    .field("Player Level", player.level, true)
                    .field("Player Rounds Won / Lost", format!("{} / {}", player_team.rounds_won, player_team.rounds_lost), true)
                )
            });

            if let Err(e) = message.await {
                println!("Failed to send message ({:?}) -> {e}", id);
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
