use std::{
    fmt::{Display, Formatter},
    process::exit,
};

use clap::Parser;
use serenity::{prelude::GatewayIntents, Client};
use tokio::{fs::File, io::AsyncReadExt, main, task};

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

const PLAYER_FILE: &str = "./players.txt";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Discord Bot Token
    #[arg(short, long)]
    token: String,

    /// Game Log Channel ID
    #[arg(short, long)]
    game_channel: Option<u64>,

    /// MMR Status Channel ID
    #[arg(short, long)]
    mmr_channel: Option<u64>,
}

#[main]
async fn main() {
    let args = Args::parse();

    let players = match File::open(PLAYER_FILE).await {
        Ok(mut f) => {
            let mut players = String::new();
            f.read_to_string(&mut players).await.unwrap();

            players
                .split('\n')
                .map(|p| p.trim())
                .map(|p| {
                    let s = p.split('#').collect::<Vec<&str>>();
                    if s.len() != 2 {
                        panic!("Invalid player tag '{p}'!");
                    }

                    PlayerData {
                        name: s[0].to_string(),
                        tag: s[1].to_string(),
                    }
                })
                .collect::<Vec<PlayerData>>()
        }
        Err(_) => {
            println!("The player file doesn't exist, creating...");
            File::create(PLAYER_FILE).await.unwrap();
            println!("Created {PLAYER_FILE}, please add all player tags (PlayerName#Tag) with a new line for each.");

            exit(0);
        }
    };

    if players.is_empty() {
        panic!("Players file was empty! No players loaded!");
    }

    println!("Loaded {} players.", players.len());

    let mut client = Client::builder(args.token, GatewayIntents::default())
        .await
        .unwrap();

    let ctx = client.cache_and_http.clone();

    if let Some(game_channel) = args.game_channel {
        task::spawn(game_tracker_thread(
            players.clone(),
            ctx.clone(),
            game_channel,
        ));

        println!("Spawned game tracker task!")
    }

    if let Some(mmr_channel) = args.mmr_channel {
        task::spawn(mmr_tracker_thread(players, ctx, mmr_channel));
        println!("Spawned mmr tracker task!")
    }

    client.start().await.expect("ERROR: Client failed to start");
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct PlayerData {
    pub name: String,
    pub tag: String,
}

impl Display for PlayerData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}#{}", self.name, self.tag))
    }
}
