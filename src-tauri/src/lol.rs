use std::time::Duration;

use crate::{
    ai::{self, GameEndRequest, GameResult, RoastRequest},
    audio,
    error::{AppError, AppResult},
    secrets::{KeyringSecretStore, SecretStore},
    settings::{AiProvider, SharedSettings},
    speech,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

const LOL_BASE_URL: &str = "https://127.0.0.1:2999";

#[derive(Debug, Deserialize)]
struct EventDataResponse {
    #[serde(rename = "Events")]
    events: Vec<LolEvent>,
}

#[derive(Debug, Clone, Deserialize)]
struct LolEvent {
    #[serde(rename = "EventID")]
    event_id: i64,
    #[serde(rename = "EventName")]
    event_name: String,
    #[serde(rename = "VictimName", default)]
    victim_name: String,
    #[serde(rename = "KillerName", default)]
    killer_name: String,
    #[serde(rename = "Result", default)]
    result: String,
}

#[derive(Debug, Deserialize)]
struct AllGameDataResponse {
    #[serde(rename = "gameData")]
    game_data: GameData,
    #[serde(rename = "allPlayers")]
    all_players: Vec<Player>,
}

#[derive(Debug, Deserialize)]
struct ActivePlayerResponse {
    #[serde(rename = "summonerName", default)]
    summoner_name: String,
    #[serde(rename = "riotId", default)]
    riot_id: String,
    #[serde(rename = "riotIdGameName", default)]
    riot_id_game_name: String,
}

#[derive(Debug, Deserialize)]
struct GameData {
    #[serde(rename = "gameTime")]
    game_time: f64,
}

#[derive(Debug, Deserialize)]
struct Player {
    #[serde(rename = "summonerName")]
    summoner_name: String,
    #[serde(rename = "championName", default)]
    champion_name: String,
    scores: Scores,
}

#[derive(Debug, Deserialize)]
struct Scores {
    kills: u32,
    deaths: u32,
    assists: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeathPayload {
    killer: String,
    death_streak: u32,
    kda: String,
    game_time_seconds: f64,
    insult: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameEndPayload {
    result: String,
    message: String,
    kda: String,
    game_time_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
struct StatusPayload {
    connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

struct PollState {
    last_event_id: i64,
    initialized_events: bool,
    death_streak: u32,
    last_connected: Option<bool>,
    last_game_time: f64,
    game_ended: bool,
}

impl Default for PollState {
    fn default() -> Self {
        Self {
            last_event_id: -1,
            initialized_events: false,
            death_streak: 0,
            last_connected: None,
            last_game_time: 0.0,
            game_ended: false,
        }
    }
}

pub async fn poll_lol(app: AppHandle, settings: SharedSettings) {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_millis(900))
        .build()
        .expect("LoL API client");

    let mut state = PollState::default();

    loop {
        let result = poll_once(&app, &client, &settings, &mut state).await;

        if let Err(error) = result {
            if state.last_connected != Some(false) {
                eprintln!("[lol] waiting for game: {error}");
            }
            emit_status(&app, &mut state, false, None);
            state.initialized_events = false;
            state.game_ended = false;
        }

        sleep(Duration::from_secs(1)).await;
    }
}

struct NewEvents {
    deaths: Vec<LolEvent>,
    game_end: Option<LolEvent>,
}

async fn poll_once(
    app: &AppHandle,
    client: &Client,
    settings: &SharedSettings,
    state: &mut PollState,
) -> AppResult<()> {
    let active_player =
        get_json::<ActivePlayerResponse>(client, "/liveclientdata/activeplayer").await?;
    let events = get_json::<EventDataResponse>(client, "/liveclientdata/eventdata").await?;

    emit_status(app, state, true, None);
    let new_events = collect_new_events(state, events.events, &active_player);

    for event in new_events.deaths {
        let snapshot = settings.read().await.clone();

        if !snapshot.overlay_enabled {
            continue;
        }

        let all_game =
            get_json::<AllGameDataResponse>(client, "/liveclientdata/allgamedata").await?;
        let death_streak = state.record_death_at_game_time(all_game.game_data.game_time);

        let kda = active_player_kda(&all_game, &active_player);
        let champion = active_player_champion(&all_game, &active_player);
        let _ = audio::play_death_sound(app, snapshot.volume);

        let killer = normalize_killer_name(&event.killer_name);

        let secrets = KeyringSecretStore;
        let Some(api_key) = secrets.get_api_key(snapshot.provider)? else {
            let message = format!("{} API key is not set.", snapshot.provider.label());
            emit_backend_message(app, true, message.clone());
            return Err(AppError::Ai(message));
        };

        let insult = match ai::generate_insult_with_settings(
            &snapshot,
            &api_key,
            RoastRequest {
                killer: &killer,
                champion: &champion,
                death_streak,
                kda: &kda,
                game_time_seconds: all_game.game_data.game_time,
            },
        )
        .await
        {
            Ok(insult) => insult,
            Err(error) => {
                emit_backend_message(app, true, error.user_message());
                continue;
            }
        };

        let _ = app.emit(
            "lol-death",
            DeathPayload {
                killer,
                death_streak,
                kda,
                game_time_seconds: all_game.game_data.game_time,
                insult: insult.clone(),
            },
        );

        if snapshot.speech_enabled && snapshot.provider == AiProvider::Gemini {
            let app = app.clone();
            let speech_key = api_key.clone();
            let speech_text = insult.clone();
            let speech_volume = snapshot.speech_volume;

            tauri::async_runtime::spawn(async move {
                match speech::synthesize_gemini_speech(&speech_key, &speech_text).await {
                    Ok(wav) => {
                        let _ = audio::play_wav_bytes(wav, speech_volume);
                    }
                    Err(error) => {
                        emit_backend_message(&app, true, error.user_message());
                    }
                }
            });
        }
    }

    if let Some(game_end_event) = new_events.game_end {
        if !state.game_ended {
            state.game_ended = true;

            let snapshot = settings.read().await.clone();
            let game_result = if game_end_event.result.eq_ignore_ascii_case("win") {
                GameResult::Win
            } else {
                GameResult::Lose
            };

            let _ = match game_result {
                GameResult::Win => audio::play_win_sound(app, snapshot.volume),
                GameResult::Lose => audio::play_lose_sound(app, snapshot.volume),
            };

            let (champion, kills, deaths, assists, game_time_seconds) =
                match get_json::<AllGameDataResponse>(client, "/liveclientdata/allgamedata").await {
                    Ok(all_game) => {
                        let champ = active_player_champion(&all_game, &active_player);
                        let (k, d, a) = active_player_scores(&all_game, &active_player);
                        (champ, k, d, a, all_game.game_data.game_time)
                    }
                    Err(_) => ("unknown champion".to_string(), 0, 0, 0, 0.0),
                };

            let kda = format!("{kills} / {deaths} / {assists}");

            let secrets = KeyringSecretStore;
            if let Ok(Some(api_key)) = secrets.get_api_key(snapshot.provider) {
                match ai::generate_game_end_message(
                    &snapshot,
                    &api_key,
                    GameEndRequest {
                        result: game_result,
                        champion: &champion,
                        kills,
                        deaths,
                        assists,
                        game_time_seconds,
                    },
                )
                .await
                {
                    Ok(message) => {
                        let result_str = if game_result == GameResult::Win { "win" } else { "lose" };
                        let _ = app.emit(
                            "lol-game-end",
                            GameEndPayload {
                                result: result_str.to_string(),
                                message,
                                kda,
                                game_time_seconds,
                            },
                        );
                    }
                    Err(error) => emit_backend_message(app, true, error.user_message()),
                }
            }
        }
    }

    Ok(())
}

async fn get_json<T: for<'de> Deserialize<'de>>(client: &Client, path: &str) -> AppResult<T> {
    let url = format!("{LOL_BASE_URL}{path}");
    let response = client.get(url).send().await.map_err(AppError::from)?;

    if !response.status().is_success() {
        return Err(AppError::Http(format!(
            "LoL API returned {}",
            response.status()
        )));
    }

    response.json::<T>().await.map_err(AppError::from)
}

fn active_player_kda(
    all_game: &AllGameDataResponse,
    active_player: &ActivePlayerResponse,
) -> String {
    all_game
        .all_players
        .iter()
        .find(|player| matches_active_player_name(&player.summoner_name, active_player))
        .map(|player| {
            format!(
                "{} / {} / {}",
                player.scores.kills, player.scores.deaths, player.scores.assists
            )
        })
        .unwrap_or_else(|| "0/0/0".to_string())
}

fn normalize_killer_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "unknown enemy".to_string()
    } else if trimmed.starts_with("Turret_") || trimmed.starts_with("Barracks_") {
        "a turret".to_string()
    } else if trimmed.starts_with("Minion_") {
        "a minion".to_string()
    } else {
        trimmed.to_string()
    }
}

fn active_player_champion(
    all_game: &AllGameDataResponse,
    active_player: &ActivePlayerResponse,
) -> String {
    all_game
        .all_players
        .iter()
        .find(|player| matches_active_player_name(&player.summoner_name, active_player))
        .map(|player| player.champion_name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "unknown champion".to_string())
}

fn active_player_scores(
    all_game: &AllGameDataResponse,
    active_player: &ActivePlayerResponse,
) -> (u32, u32, u32) {
    all_game
        .all_players
        .iter()
        .find(|player| matches_active_player_name(&player.summoner_name, active_player))
        .map(|p| (p.scores.kills, p.scores.deaths, p.scores.assists))
        .unwrap_or((0, 0, 0))
}

fn emit_status(app: &AppHandle, state: &mut PollState, connected: bool, message: Option<String>) {
    if state.last_connected == Some(connected) {
        return;
    }

    state.last_connected = Some(connected);
    let _ = app.emit("lol-status", StatusPayload { connected, message });
}

fn emit_backend_message(app: &AppHandle, connected: bool, message: String) {
    let _ = app.emit(
        "lol-status",
        StatusPayload {
            connected,
            message: Some(message),
        },
    );
}

fn matches_active_player_name(candidate: &str, active_player: &ActivePlayerResponse) -> bool {
    let candidate = normalize_player_name(candidate);

    if candidate.is_empty() {
        return false;
    }

    [
        active_player.summoner_name.as_str(),
        active_player.riot_id.as_str(),
        active_player.riot_id_game_name.as_str(),
    ]
    .into_iter()
    .map(normalize_player_name)
    .any(|name| !name.is_empty() && name == candidate)
}

fn normalize_player_name(name: &str) -> String {
    let trimmed = name.trim().trim_matches('"');

    if trimmed.is_empty() {
        return String::new();
    }

    trimmed
        .split('#')
        .next()
        .unwrap_or(trimmed)
        .trim()
        .to_ascii_lowercase()
}

impl PollState {
    fn reset_game(&mut self) {
        self.last_event_id = -1;
        self.death_streak = 0;
        self.last_game_time = 0.0;
        self.game_ended = false;
    }

    fn record_death_at_game_time(&mut self, game_time: f64) -> u32 {
        if game_time + 5.0 < self.last_game_time {
            self.death_streak = 0;
        }

        self.last_game_time = game_time;
        self.death_streak += 1;
        self.death_streak
    }
}

fn collect_new_events(
    state: &mut PollState,
    events: Vec<LolEvent>,
    active_player: &ActivePlayerResponse,
) -> NewEvents {
    let max_event_id = events.iter().map(|event| event.event_id).max().unwrap_or(0);

    if state.initialized_events && max_event_id < state.last_event_id {
        state.reset_game();
    }

    if !state.initialized_events {
        state.last_event_id = max_event_id;
        state.initialized_events = true;
        return NewEvents { deaths: Vec::new(), game_end: None };
    }

    let mut pending = events
        .into_iter()
        .filter(|event| event.event_id > state.last_event_id)
        .collect::<Vec<_>>();

    pending.sort_by_key(|event| event.event_id);

    let mut deaths = Vec::new();
    let mut game_end = None;

    for event in pending {
        state.last_event_id = state.last_event_id.max(event.event_id);

        if event.event_name == "ChampionKill"
            && matches_active_player_name(&event.victim_name, active_player)
        {
            deaths.push(event);
        } else if event.event_name == "GameEnd" {
            game_end = Some(event);
        }
    }

    NewEvents { deaths, game_end }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_new_events, matches_active_player_name, ActivePlayerResponse, LolEvent, PollState,
    };

    fn active_player() -> ActivePlayerResponse {
        ActivePlayerResponse {
            summoner_name: "Flamed".to_string(),
            riot_id: "Flamed#EUW".to_string(),
            riot_id_game_name: "Flamed".to_string(),
        }
    }

    fn champion_kill(event_id: i64, victim_name: &str) -> LolEvent {
        LolEvent {
            event_id,
            event_name: "ChampionKill".to_string(),
            victim_name: victim_name.to_string(),
            killer_name: "Annie Bot".to_string(),
            result: String::new(),
        }
    }

    #[test]
    fn duplicate_events_do_not_trigger_duplicate_deaths() {
        let mut state = PollState::default();
        let active_player = active_player();

        let initial = collect_new_events(
            &mut state,
            vec![champion_kill(10, "Flamed")],
            &active_player,
        );
        assert!(initial.deaths.is_empty());

        let first = collect_new_events(
            &mut state,
            vec![champion_kill(10, "Flamed"), champion_kill(11, "Flamed")],
            &active_player,
        );
        let duplicate = collect_new_events(
            &mut state,
            vec![champion_kill(10, "Flamed"), champion_kill(11, "Flamed")],
            &active_player,
        );

        assert_eq!(first.deaths.len(), 1);
        assert!(duplicate.deaths.is_empty());
    }

    #[test]
    fn lower_event_ids_reset_new_game_state() {
        let mut state = PollState::default();
        let active_player = active_player();

        collect_new_events(&mut state, vec![champion_kill(10, "Flamed")], &active_player);
        state.record_death_at_game_time(100.0);

        let result =
            collect_new_events(&mut state, vec![champion_kill(1, "Flamed")], &active_player);

        assert_eq!(state.death_streak, 0);
        assert_eq!(state.last_game_time, 0.0);
        assert_eq!(result.deaths.len(), 1);
    }

    #[test]
    fn active_player_matching_handles_riot_ids_and_tags() {
        let active_player = active_player();

        assert!(matches_active_player_name("Flamed#EUW", &active_player));
        assert!(matches_active_player_name("\"Flamed\"", &active_player));
        assert!(matches_active_player_name("Flamed", &active_player));
        assert!(!matches_active_player_name(
            "OtherPlayer#EUW",
            &active_player
        ));
    }
}
