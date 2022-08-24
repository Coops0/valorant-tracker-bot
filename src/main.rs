use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    time::Duration,
};

use anyhow::bail;
use reqwest::get;
use serenity::{
    model::id::ChannelId, model::prelude::UserId, prelude::GatewayIntents, utils::Color, Client,
};
use tokio::{task, time::sleep};

use crate::hendrix_matches_response::{HendrixMatchesResponse, MatchDatum, Player, TeamEnum};
use crate::hendrix_mmr_response::{HendrixMmrResponse, MmrDatum};

mod hendrix_matches_response;
mod hendrix_mmr_response;

const BASE_URL: &str = "https://api.henrikdev.xyz";
const MATCH_URL: &str = "/valorant/v3/matches/na";
const MMR_HISTORY_URL: &str = "/valorant/v1/mmr-history/na";

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
struct PlayerData<'a> {
    name: &'a str,
    tag: &'a str,
    discord_id: UserId,
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
        }
    }
}

impl Display for PlayerData<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}#{}", self.name, self.tag))
    }
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

#[tokio::main]
async fn main() {
    let players = vec![
        PlayerData::new("finicky", "8260", 391061411813523474),
        PlayerData::new("leirbag", "0001", 430013185056178176),
        PlayerData::new("mvh", "0001", 412278960458694666),
        PlayerData::new("Chaz", "HEHR", 408054716723888138),
        PlayerData::new("rvulyobdeifitreC", "0001", 412278960458694666),
        PlayerData::new("jeremyawesome", "NA1", 406956734154932235),
        PlayerData::new("bakon", "8597", 435920046238466049),
    ];

    let mut last_games = players
        .iter()
        .map(|p| (p, LastData::new()))
        .collect::<Vec<(&PlayerData, LastData)>>();

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

    let current_ctx = ctx.clone();
    let our_players = players.clone();
    task::spawn(async move {
        let channel = ChannelId(1011839991376248902);

        let mut mmrs = our_players
            .iter()
            .map(|p| (p, None))
            .collect::<HashMap<&PlayerData, Option<MmrDatum>>>();

        let mut message = match channel.messages(&current_ctx.http, |b| b).await {
            Ok(mut m) if !m.is_empty() => m.remove(0),
            _ => channel
                .send_message(&current_ctx.http, |m| m.content("wait bruh"))
                .await
                .expect("Failed to send initial message"),
        };

        loop {
            let mut was_changed = false;

            for (player, old_data) in mmrs.clone() {
                let mmr = match lookup_player_mmr(player.name, player.tag).await {
                    Ok(mmr) => mmr,
                    Err(e) => {
                        println!("ERROR: Failed to get MMR for {player} -> {e}");
                        continue;
                    }
                };

                if old_data.map(|m| m.elo).unwrap_or_default() != mmr.elo {
                    was_changed = true;
                    println!("INFO: Detected MMR change in {player}.");
                }

                mmrs.insert(player, Some(mmr));
            }

            if was_changed {
                let mut content = "```yml".to_string();

                let mut alphabetically = mmrs
                    .iter()
                    .collect::<Vec<(&&PlayerData, &Option<MmrDatum>)>>();

                dbg!(&alphabetically
                    .iter()
                    .map(|(a, _)| a.name)
                    .collect::<Vec<&str>>());
                alphabetically.sort_by(|(a, _), (b, _)| {
                    a.name
                        .first_char()
                        .to_ascii_lowercase()
                        .cmp(&b.name.first_char().to_ascii_lowercase())
                });
                dbg!(&alphabetically
                    .iter()
                    .map(|(a, _)| a.name)
                    .collect::<Vec<&str>>());

                for (player, data) in alphabetically {
                    if let Some(data) = data {
                        content = format!(
                            "{content}\n{}: {} @ {} MMR",
                            player.name, data.current_tier_patched, data.ranking_in_tier
                        );
                    }
                }

                content = format!("{content}\n```");
                match message
                    .edit(&current_ctx.http, |m| m.content(content))
                    .await
                {
                    Err(e) => println!("ERROR: Failed to update mmr message -> {e}"),
                    Ok(_) => println!("INFO: Successfully updated MMR message."),
                }
            }

            sleep(Duration::from_secs(60)).await;
        }
    });

    loop {
        // Clone so we can mutate it in the loop
        for (i, (id, last_data)) in last_games.clone().iter().enumerate() {
            let PlayerData {
                name,
                tag,
                discord_id,
            } = id;

            let mut last_data = last_data.clone();
            let LastData {
                last_game_id,
                last_mmr_change_timestamp,
            } = last_data;

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
                    "Average Combat Core",
                    player_stats.score / game.rounds.len() as i64,
                ),
                field("Player Rank", &player.current_tier_patched),
                field("Map", &metadata.map),
            ];

            match lookup_player_mmr(name, tag).await {
                Ok(mmr) => {
                    last_data.last_mmr_change_timestamp = Some(mmr.date_raw);
                    last_games[i] = (id, last_data);

                    if last_mmr_change_timestamp.unwrap_or_default() == mmr.date_raw {
                        println!("INFO: Last MMR is same as newest for {id}");
                    } else {
                        fields.push(field(
                            "MMR Change",
                            format!(
                                "{}{}",
                                if mmr.mmr_change_to_last_game > 0 {
                                    "+"
                                } else {
                                    ""
                                },
                                mmr.mmr_change_to_last_game
                            ),
                        ));
                        fields.push(field("Elo", mmr.ranking_in_tier));
                    }
                }
                Err(e) => println!("ERROR: Failed to get mmr change for {id} -> {e}"),
            };

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
                Err(e) => println!("ERROR: Failed to send match message ({id}) -> {e}"),
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn lookup_player_matches(name: &str, tag: &str) -> anyhow::Result<MatchDatum> {
    let response = get(format!(
        "{BASE_URL}{MATCH_URL}/{name}/{tag}?filter=competitive&size=1"
    ))
    .await?;
    let parsed = response.json::<HendrixMatchesResponse>().await?;

    if parsed.status != 200 {
        bail!("got status of {} instead of 200", parsed.status);
    }

    match parsed.data {
        Some(mut d) if !d.is_empty() => Ok(d.remove(0)),
        _ => bail!("no matches found"),
    }
}

async fn lookup_player_mmr(name: &str, tag: &str) -> anyhow::Result<MmrDatum> {
    let response = get(format!("{BASE_URL}{MMR_HISTORY_URL}/{name}/{tag}?size=1"))
        .await?
        .json::<HendrixMmrResponse>()
        .await?;

    if response.status != 200 {
        bail!("got status of {} instead of 200", response.status);
    }

    match response.data {
        Some(mut d) if !d.is_empty() => Ok(d.remove(0)),
        _ => bail!("no mmr found"),
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

trait FirstChar {
    fn first_char(&self) -> char;
}

impl FirstChar for str {
    fn first_char(&self) -> char {
        self.chars().take(1).last().unwrap()
    }
}
