use std::{sync::Arc, time::Duration};

use anyhow::bail;
use reqwest::get;
use serenity::{model::id::ChannelId, model::Timestamp, utils::Color, CacheAndHttp};
use tokio::time::sleep;

use crate::{
    mmr_tracker::lookup_player_mmr, HendrixMatchesResponse, MatchDatum, Player, PlayerData,
    TeamEnum, BASE_URL, MATCH_URL,
};

pub async fn game_tracker_thread<T: Into<ChannelId>>(
    players: Vec<PlayerData<'_>>,
    ctx: Arc<CacheAndHttp>,
    channel: T,
) {
    let channel = channel.into();

    let mut last_games = players
        .iter()
        .map(|p| (p, LastData::new()))
        .collect::<Vec<(&PlayerData, LastData)>>();

    loop {
        // Clone so we can mutate it in the loop
        for (i, (id, last_data)) in last_games.clone().iter().enumerate() {
            let PlayerData { name, tag, .. } = id;

            let mut last_data = last_data.clone();
            let LastData { last_game_id, .. } = last_data;

            let game = match lookup_player_matches(name, tag).await {
                Ok(o) => o,
                Err(e) => {
                    println!("ERROR: Failed to get player info for {id} -> {e}");
                    continue;
                }
            };

            let metadata = &game.metadata;
            let newest_last_game_id = metadata.match_id.clone();

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

            last_data.last_game_id = Some(newest_last_game_id.clone());
            last_games[i] = (id, last_data.clone());

            if let Some(last_stored_game) = last_game_id {
                if last_stored_game == newest_last_game_id {
                    println!("INFO: Last stored game is same as newest for {id}");
                    continue;
                }
            } else {
                println!("INFO: No game stored, so no need to send match message for {id}");
                continue;
            }

            let kd = format!("{:.2}", calculate_kd(player));

            let mut kd_ranking = game
                .players
                .all_players
                .iter()
                .map(|p| (p, calculate_kd(p)))
                .collect::<Vec<(&Player, f64)>>();
            kd_ranking.sort_by(|(_, akb), (_, bkb)| bkb.partial_cmp(akb).unwrap());

            let position = kd_ranking
                .iter()
                .enumerate()
                .find(|(_, (p, _))| p.puuid == player.puuid)
                .unwrap() // Should NEVER fail
                .0
                + 1; // It's an index so add one

            // this is cancerous but not really a better way to do this that doesn't require just moving it into the other file
            let player_team = if player.team == TeamEnum::Red {
                &game.teams.red
            } else {
                &game.teams.blue
            };

            let mut fields = vec![
                field("Map", &metadata.map),
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
                field(
                    "Head Shot Percentage",
                    format!("{}%", calculate_headshot_percentage(player) as i64),
                ),
                field(
                    "Average Combat Score",
                    player_stats.score / game.rounds.len() as i64,
                ),
            ];

            if let Some(playtime) = player.session_playtime.minutes {
                fields.push(field("Session Playtime", format!("{}min", playtime)));
            }

            let behavior = &player.behavior;
            if behavior.afk_rounds > 0_f64 {
                fields.push(field("AFK Rounds", behavior.afk_rounds));
            }

            if behavior.rounds_in_spawn > 0_f64 {
                fields.push(field("Rounds in Spawn", behavior.rounds_in_spawn));
            }

            if player.party_id.is_some() {
                let partied_with = game
                    .players
                    .all_players
                    .iter()
                    .filter(|p| p.party_id == player.party_id && p.name != player.name)
                    .map(|p| format!("{}#{}", p.name, p.tag))
                    .collect::<Vec<String>>();

                if !partied_with.is_empty() {
                    fields.push(field("Partied With", partied_with.join(", ")))
                }
            }

            if let Some(mmr_fields) = get_mmr_fields(id, &mut last_data).await {
                for field in mmr_fields {
                    fields.push(field);
                }

                last_games[i] = (id, last_data);
            }

            let message = channel.send_message(&ctx.http, |m| {
                m.embed(|e| {
                    e.title(format!("{}'s Game on {}", id.username(), metadata.map))
                        .color(if player_team.has_won {
                            Color::DARK_GREEN
                        } else {
                            Color::DARK_RED
                        })
                        .image(&player.assets.card.wide)
                        .thumbnail(&player.assets.agent.small)
                        .timestamp(Timestamp::from_unix_timestamp(game.metadata.game_start).unwrap_or_else(|_| Timestamp::now()))
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
                Err(e) => println!("ERROR: Failed to send match message ({id}) -> {e}"),
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn get_mmr_fields(
    player: &PlayerData<'_>,
    last_data: &mut LastData,
) -> Option<Vec<(String, String, bool)>> {
    let mmr = lookup_player_mmr(player.name, player.tag).await.ok()?;

    let last_mmr_change_timestamp = last_data.last_mmr_change_timestamp.unwrap_or_default();

    last_data.last_mmr_change_timestamp = Some(mmr.date_raw);
    if last_mmr_change_timestamp == mmr.date_raw {
        return None;
    }

    let op = if mmr.mmr_change_to_last_game > 0 {
        "+"
    } else {
        ""
    };

    Some(vec![
        field("MMR Change", format!("{op}{}", mmr.mmr_change_to_last_game)),
        field(
            "Rank",
            format!("{} @ {} MMR", mmr.current_tier_patched, mmr.ranking_in_tier),
        ),
    ])
}

#[derive(Clone)]
struct LastData {
    last_game_id: Option<String>,
    last_mmr_change_timestamp: Option<i64>,
}

impl LastData {
    fn new() -> Self {
        Self {
            last_game_id: None,
            last_mmr_change_timestamp: None,
        }
    }
}

fn calculate_kd(player: &Player) -> f64 {
    player.stats.kills as f64 / player.stats.deaths as f64
}

fn calculate_headshot_percentage(player: &Player) -> f64 {
    let all_shots =
        (player.stats.head_shots + player.stats.body_shots + player.stats.leg_shots) as f64;

    (player.stats.head_shots as f64 / all_shots) * 100_f64
}

#[inline]
fn field<A: ToString, B: ToString>(key: A, value: B) -> (String, String, bool) {
    (key.to_string(), value.to_string(), true)
}

async fn lookup_player_matches(name: &str, tag: &str) -> anyhow::Result<MatchDatum> {
    let response = get(format!(
        "{BASE_URL}{MATCH_URL}/{name}/{tag}?filter=competitive&size=1"
    ))
    .await?
    .json::<HendrixMatchesResponse>()
    .await?;

    if response.status != 200 {
        bail!("got status of {} instead of 200", response.status);
    }

    match response.data {
        Some(mut d) if !d.is_empty() => Ok(d.remove(0)),
        _ => bail!("no matches found"),
    }
}
