use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use reqwest::get;
use serenity::{model::id::ChannelId, CacheAndHttp};
use tokio::time::sleep;

use crate::{HendrixMmrResponse, MmrDatum, PlayerData, BASE_URL, MMR_HISTORY_URL};

pub async fn mmr_tracker_thread<T: Into<ChannelId>>(
    players: Vec<PlayerData>,
    ctx: Arc<CacheAndHttp>,
    channel: T,
) {
    let channel = channel.into();

    let mut mmrs = players
        .iter()
        .map(|p| (p, None))
        .collect::<HashMap<&PlayerData, Option<MmrDatum>>>();

    let mut message = match channel.messages(&ctx.http, |b| b).await {
        Ok(mut m) if !m.is_empty() => m.remove(0),
        _ => channel
            .send_message(&ctx.http, |m| m.content("wait bruh"))
            .await
            .expect("Failed to send initial message"),
    };

    loop {
        let mut was_changed = false;

        for (player, old_data) in mmrs.clone() {
            let mmr = match lookup_player_mmr(&player.name, &player.tag).await {
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
            let mut content = String::new();

            let mut sorted = mmrs
                .iter()
                .filter(|(_, d)| d.is_some())
                .map(|(p, d)| (p, d.as_ref().unwrap()))
                .collect::<Vec<(&&PlayerData, &MmrDatum)>>();
            sorted.sort_by(|(_, a), (_, b)| b.elo.cmp(&a.elo));

            for (player, data) in sorted {
                content = format!(
                    "{content}\n{} -> `{} @ {} MMR`",
                    player.name, data.current_tier_patched, data.ranking_in_tier
                );
            }

            match message.edit(&ctx.http, |m| m.content(content)).await {
                Ok(_) => println!("SUCCESS: Successfully updated MMR message."),
                Err(e) => println!("ERROR: Failed to update mmr message -> {e}"),
            }
        }

        sleep(Duration::from_secs(60)).await;
    }
}

pub async fn lookup_player_mmr(name: &str, tag: &str) -> Result<MmrDatum> {
    let response = get(format!("{BASE_URL}{MMR_HISTORY_URL}/{name}/{tag}?size=1"))
        .await?
        .json::<HendrixMmrResponse>()
        .await?;

    if response.status != 200 {
        bail!(
            "got status of {} instead of 200 -> {:?}",
            response.status,
            response
        );
    }

    match response.data {
        Some(mut d) if !d.is_empty() => Ok(d.remove(0)),
        _ => bail!("no mmr found"),
    }
}
