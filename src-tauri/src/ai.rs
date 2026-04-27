use crate::{
    error::{AppError, AppResult},
    settings::{AiProvider, InsultPreset, ModelTier, Settings},
};
use genai::{
    adapter::AdapterKind,
    chat::{ChatMessage, ChatOptions, ChatRequest, ChatResponseFormat, ReasoningEffort},
    resolver::AuthData,
    Client, ModelIden,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderModel {
    pub tier: ModelTier,
    pub label: &'static str,
    pub model: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderOption {
    pub id: AiProvider,
    pub label: &'static str,
    pub models: &'static [ProviderModel],
}

#[derive(Debug, Clone)]
pub struct RoastRequest<'a> {
    pub killer: &'a str,
    pub champion: &'a str,
    pub death_streak: u32,
    pub kda: &'a str,
    pub game_time_seconds: f64,
}

struct RoastPrompt {
    system: String,
    user: String,
}

const GEMINI_MODELS: &[ProviderModel] = &[
    ProviderModel {
        tier: ModelTier::Frontier,
        label: "Frontier - Gemini 2.5 Pro",
        model: "gemini-2.5-pro",
    },
    ProviderModel {
        tier: ModelTier::Mid,
        label: "Mid - Gemini 2.5 Flash",
        model: "gemini-2.5-flash",
    },
    ProviderModel {
        tier: ModelTier::Budget,
        label: "Budget - Gemini 2.5 Flash-Lite",
        model: "gemini-2.5-flash-lite",
    },
];

const OPENAI_MODELS: &[ProviderModel] = &[
    ProviderModel {
        tier: ModelTier::Frontier,
        label: "Frontier - GPT-5.2",
        model: "gpt-5.2",
    },
    ProviderModel {
        tier: ModelTier::Mid,
        label: "Mid - GPT-5 Mini",
        model: "gpt-5-mini",
    },
    ProviderModel {
        tier: ModelTier::Budget,
        label: "Budget - GPT-5 Nano",
        model: "gpt-5-nano",
    },
];

const ANTHROPIC_MODELS: &[ProviderModel] = &[
    ProviderModel {
        tier: ModelTier::Frontier,
        label: "Frontier - Claude Opus 4.5",
        model: "claude-opus-4-5",
    },
    ProviderModel {
        tier: ModelTier::Mid,
        label: "Mid - Claude Sonnet 4.5",
        model: "claude-sonnet-4-5",
    },
    ProviderModel {
        tier: ModelTier::Budget,
        label: "Budget - Claude Haiku 4.5",
        model: "claude-haiku-4-5",
    },
];

pub const PROVIDERS: &[ProviderOption] = &[
    ProviderOption {
        id: AiProvider::Gemini,
        label: "Gemini",
        models: GEMINI_MODELS,
    },
    ProviderOption {
        id: AiProvider::OpenAi,
        label: "OpenAI",
        models: OPENAI_MODELS,
    },
    ProviderOption {
        id: AiProvider::Anthropic,
        label: "Anthropic",
        models: ANTHROPIC_MODELS,
    },
];

pub fn provider_options() -> &'static [ProviderOption] {
    PROVIDERS
}

pub fn default_model_for_provider(provider: AiProvider) -> &'static str {
    model_options_for_provider(provider)
        .iter()
        .find(|model| model.tier == ModelTier::Budget)
        .or_else(|| model_options_for_provider(provider).first())
        .map(|model| model.model)
        .unwrap_or("gemini-2.5-flash-lite")
}

pub fn model_options_for_provider(provider: AiProvider) -> &'static [ProviderModel] {
    match provider {
        AiProvider::Gemini => GEMINI_MODELS,
        AiProvider::OpenAi => OPENAI_MODELS,
        AiProvider::Anthropic => ANTHROPIC_MODELS,
    }
}

pub fn normalize_provider_model(provider: AiProvider, model: String) -> String {
    if model_options_for_provider(provider)
        .iter()
        .any(|option| option.model == model)
    {
        model
    } else {
        default_model_for_provider(provider).to_string()
    }
}

fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn tone_for_streak(streak: u32) -> &'static str {
    match streak {
        0 | 1 => "first death: one sharp pointed jab — clean and targeted",
        2 => "second death: contemptuous and losing patience, make it sting more",
        3 | 4 => "third or fourth death: openly disgusted, mock their very existence in this game",
        _ => "five-plus deaths: full meltdown — they have destroyed this game, make them feel completely hopeless",
    }
}

fn preset_instruction(preset: InsultPreset) -> &'static str {
    match preset {
        InsultPreset::Warmup => "L1 warmup: clever jab, real bite, no heavy cruelty.",
        InsultPreset::Salty => "L2 salty: mean, annoyed, mocking; mild profanity if uncensored.",
        InsultPreset::Brutal => "L3 brutal: harsh gameplay humiliation, open contempt; profanity welcome and varied.",
        InsultPreset::Nuclear => {
            "L4 nuclear: soul-destroying contempt, zero mercy. Be coldly precise and surgical — clinical contempt cuts deeper than screaming. If uncensored, profanity is mandatory — use it naturally and vary it widely, never recycle the same words across insults."
        }
    }
}

fn censorship_instruction(enabled: bool) -> &'static str {
    if enabled {
        "on: mask profanity with *, e.g. f***ing, s**t, d*****t, p***y. No raw profanity."
    } else {
        "off: uncensored profanity allowed by level."
    }
}

fn build_roast_prompt(settings: &Settings, request: &RoastRequest<'_>) -> RoastPrompt {
    let level = {
        let base = preset_instruction(settings.insult_preset);
        if settings.insult_preset == InsultPreset::Nuclear && request.death_streak >= 5 {
            format!("{base} FEEDING FRENZY: nuclear preset combined with 5+ consecutive deaths — absolute rock bottom, zero restraint, maximum contempt.")
        } else {
            base.to_string()
        }
    };

    let system = format!(
        "League death roast. JSON only: {{\"roast\":\"...\"}}. One sentence, 8-18 words, <=110 chars. Target: the PLAYER who just died — roast their gameplay, not the killer's. The killer's name is flavor/context only, never the butt of the joke. Roast angle — choose one per insult: positioning, mechanics, game sense, champion mastery, ego gap, decision-making, teamfighting, macro, mental, or cause of death. Vary angle and vocabulary every call. KDA context: 0-1 kills + 3+ deaths = feeding hard; late-game deaths after a lead = throwing. Champion context: ADC dying = positioning, carry dying = decisions, melee dying to poke = awareness gap — use these if they sharpen the roast. Avoid coaching, sympathy, positivity, labels, lists, and 'skill issue'. Never use protected-class hate, slurs, threats, self-harm, or doxxing.\nLevel: {}\nStreak: {}\nCensor: {}",
        level,
        tone_for_streak(request.death_streak),
        censorship_instruction(settings.censorship_enabled)
    );
    let user = format!(
        "champion={}; killer={}; kda={}; time={}; streak={}. Return JSON.",
        request.champion,
        request.killer,
        request.kda,
        format_time(request.game_time_seconds),
        request.death_streak
    );

    RoastPrompt { system, user }
}

fn clean_output(text: &str) -> String {
    text.trim()
        .trim_matches('"')
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_for_comparison(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn extract_first_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in text[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }

            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);

                if depth == 0 {
                    let end = start + idx + ch.len_utf8();
                    return Some(&text[start..end]);
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_roast_from_response(text: &str) -> Option<String> {
    let json_text = extract_first_json_object(text.trim())?;
    let value = serde_json::from_str::<Value>(json_text).ok()?;
    let roast = value.get("roast")?.as_str()?;
    Some(clean_output(roast))
}

fn is_bad_roast(roast: &str, killer: &str) -> bool {
    let trimmed = roast.trim();

    if trimmed.len() < 35 {
        return true;
    }

    if trimmed.split_whitespace().count() < 4 {
        return true;
    }

    let normalized_roast = normalize_for_comparison(trimmed);
    let normalized_killer = normalize_for_comparison(killer);
    let normalized_killer_bot = format!("{normalized_killer}bot");

    normalized_roast.is_empty()
        || normalized_roast == normalized_killer
        || normalized_roast == normalized_killer_bot
}

pub async fn generate_insult_with_settings(
    settings: &Settings,
    api_key: &str,
    request: RoastRequest<'_>,
) -> AppResult<String> {
    let api_key = api_key.trim();

    if api_key.is_empty() {
        return Err(AppError::Ai(format!(
            "{} API key is not set.",
            settings.provider.label()
        )));
    }

    let client = client_for_provider(settings.provider, api_key.to_string());
    let model_name = model_name_for_genai(settings);
    let options = ChatOptions::default()
        .with_temperature(0.85)
        .with_max_tokens(500)
        .with_response_format(ChatResponseFormat::JsonMode)
        .with_reasoning_effort(ReasoningEffort::None);

    for attempt in 0..2u8 {
        let prompt = build_roast_prompt(settings, &request);
        let chat_req = ChatRequest::new(vec![
            ChatMessage::system(prompt.system),
            ChatMessage::user(prompt.user),
        ]);

        let chat_res = client
            .exec_chat(&model_name, chat_req, Some(&options))
            .await
            .map_err(|error| AppError::Ai(error.to_string()))?;
        let text = chat_res
            .first_text()
            .ok_or_else(|| AppError::Ai("AI provider returned no text.".to_string()))?;

        let cleaned = extract_roast_from_response(text)
            .or_else(|| Some(clean_output(text)).filter(|value| !value.is_empty()))
            .ok_or_else(|| AppError::Ai("AI provider returned an unusable roast.".to_string()))?;

        if is_bad_roast(&cleaned, request.killer) {
            if attempt == 0 {
                continue;
            }
            return Err(AppError::Ai(
                "AI provider returned a weak or unusable roast.".to_string(),
            ));
        }

        return Ok(cleaned);
    }

    Err(AppError::Ai(
        "AI provider returned a weak or unusable roast.".to_string(),
    ))
}

pub async fn test_api_key(provider: AiProvider, api_key: &str) -> AppResult<bool> {
    let api_key = api_key.trim();

    if api_key.is_empty() {
        return Err(AppError::Ai(format!(
            "{} API key is empty.",
            provider.label()
        )));
    }

    let settings = Settings {
        provider,
        selected_model: default_model_for_provider(provider).to_string(),
        ..Settings::default()
    }
    .normalize();
    let client = client_for_provider(provider, api_key.to_string());
    let model_name = model_name_for_genai(&settings);
    let chat_req = ChatRequest::from_user("Return exactly: ok");
    let options = ChatOptions::default()
        .with_temperature(0.0)
        .with_max_tokens(8)
        .with_reasoning_effort(ReasoningEffort::None);

    client
        .exec_chat(&model_name, chat_req, Some(&options))
        .await
        .map_err(|error| {
            AppError::Ai(format!("{} API key test failed: {error}", provider.label()))
        })?;

    Ok(true)
}

fn client_for_provider(provider: AiProvider, api_key: String) -> Client {
    let auth_resolver = genai::resolver::AuthResolver::from_resolver_fn(
        move |model_iden: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
            let expected_adapter = match provider {
                AiProvider::Gemini => AdapterKind::Gemini,
                AiProvider::OpenAi => AdapterKind::OpenAI,
                AiProvider::Anthropic => AdapterKind::Anthropic,
            };

            if model_iden.adapter_kind == expected_adapter {
                Ok(Some(AuthData::from_single(api_key.clone())))
            } else {
                Ok(None)
            }
        },
    );

    Client::builder().with_auth_resolver(auth_resolver).build()
}

fn model_name_for_genai(settings: &Settings) -> String {
    settings.selected_model.clone()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameResult {
    Win,
    Lose,
}

pub struct GameEndRequest<'a> {
    pub result: GameResult,
    pub champion: &'a str,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub game_time_seconds: f64,
}

fn build_game_end_prompt(settings: &Settings, request: &GameEndRequest<'_>) -> RoastPrompt {
    let game_time = format_time(request.game_time_seconds);

    let system = match request.result {
        GameResult::Win => format!(
            "League win roast. JSON only: {{\"roast\":\"...\"}}. One sentence, <=120 chars. The player won — write a sarcastic, backhanded congratulation. Be genuinely surprised they pulled it off. Reference their champion and KDA to mock their performance while technically acknowledging the win — a good KDA means they got carried, a bad one means they barely survived. Never be sincere or wholesome. Censor: {}.",
            censorship_instruction(settings.censorship_enabled)
        ),
        GameResult::Lose => format!(
            "League loss verdict. JSON only: {{\"roast\":\"...\"}}. One sentence, <=120 chars. The player just lost — deliver a brutal final verdict on their overall game, not any single death. Use their champion and KDA to make it specific: many deaths = they inted the game away, few kills = they were useless, both = hopeless. Level: {}. Censor: {}.",
            preset_instruction(settings.insult_preset),
            censorship_instruction(settings.censorship_enabled)
        ),
    };

    let user = format!(
        "champion={}; k/d/a={}/{}/{}; time={}. Return JSON.",
        request.champion, request.kills, request.deaths, request.assists, game_time
    );

    RoastPrompt { system, user }
}

pub async fn generate_game_end_message(
    settings: &Settings,
    api_key: &str,
    request: GameEndRequest<'_>,
) -> AppResult<String> {
    let api_key = api_key.trim();

    if api_key.is_empty() {
        return Err(AppError::Ai(format!(
            "{} API key is not set.",
            settings.provider.label()
        )));
    }

    let client = client_for_provider(settings.provider, api_key.to_string());
    let model_name = model_name_for_genai(settings);
    let options = ChatOptions::default()
        .with_temperature(0.9)
        .with_max_tokens(200)
        .with_response_format(ChatResponseFormat::JsonMode)
        .with_reasoning_effort(ReasoningEffort::None);

    let prompt = build_game_end_prompt(settings, &request);
    let chat_req = ChatRequest::new(vec![
        ChatMessage::system(prompt.system),
        ChatMessage::user(prompt.user),
    ]);

    let chat_res = client
        .exec_chat(&model_name, chat_req, Some(&options))
        .await
        .map_err(|e| AppError::Ai(e.to_string()))?;

    let text = chat_res
        .first_text()
        .ok_or_else(|| AppError::Ai("AI returned no text.".to_string()))?;

    extract_roast_from_response(text)
        .or_else(|| Some(clean_output(text)).filter(|v| !v.is_empty()))
        .ok_or_else(|| AppError::Ai("AI returned an unusable game-end message.".to_string()))
}

#[cfg(test)]
mod tests {
    use super::{
        build_roast_prompt, censorship_instruction, default_model_for_provider,
        extract_roast_from_response, is_bad_roast, model_options_for_provider,
        normalize_provider_model, preset_instruction, RoastRequest,
    };
    use crate::settings::{AiProvider, InsultPreset, Settings};

    #[test]
    fn extracts_json_after_preface_text() {
        let response = r#"Here is the JSON requested {"roast":"Annie Bot farmed you so hard the lane should file a restraining order."}"#;

        assert_eq!(
            extract_roast_from_response(response).as_deref(),
            Some("Annie Bot farmed you so hard the lane should file a restraining order.")
        );
    }

    #[test]
    fn rejects_killer_name_only_outputs() {
        assert!(is_bad_roast("Darius", "Darius Bot"));
        assert!(is_bad_roast("Annie Bot", "Annie Bot"));
    }

    #[test]
    fn rejects_roasts_under_min_length() {
        assert!(is_bad_roast("You suck so bad", "Darius"));
        assert!(!is_bad_roast(
            "You walked straight into that like a braindead minion.",
            "Darius"
        ));
    }

    #[test]
    fn exposes_three_models_per_provider() {
        for provider in [
            AiProvider::Gemini,
            AiProvider::OpenAi,
            AiProvider::Anthropic,
        ] {
            assert_eq!(model_options_for_provider(provider).len(), 3);
        }
    }

    #[test]
    fn invalid_model_defaults_to_provider_budget_model() {
        assert_eq!(
            normalize_provider_model(AiProvider::OpenAi, "unknown".to_string()),
            default_model_for_provider(AiProvider::OpenAi)
        );
    }

    #[test]
    fn insult_presets_progress_in_intensity() {
        assert!(preset_instruction(InsultPreset::Warmup).contains("L1"));
        assert!(preset_instruction(InsultPreset::Salty).contains("L2"));
        assert!(preset_instruction(InsultPreset::Brutal).contains("L3"));
        assert!(preset_instruction(InsultPreset::Nuclear).contains("L4"));
        assert!(preset_instruction(InsultPreset::Nuclear).contains("soul-destroying"));
        assert!(preset_instruction(InsultPreset::Nuclear).contains("surgical"));
        assert!(preset_instruction(InsultPreset::Nuclear).contains("profanity"));
    }

    #[test]
    fn censorship_is_prompted_instead_of_post_processed() {
        assert!(censorship_instruction(true).contains("mask profanity"));
        assert!(censorship_instruction(true).contains("No raw profanity"));
        assert!(censorship_instruction(false).contains("uncensored profanity"));
    }

    #[test]
    fn builds_compact_roast_prompt_with_context_and_safety_rules() {
        let settings = Settings {
            insult_preset: InsultPreset::Nuclear,
            censorship_enabled: false,
            ..Settings::default()
        };
        let prompt = build_roast_prompt(
            &settings,
            &RoastRequest {
                killer: "Annie Bot",
                champion: "Yasuo",
                death_streak: 5,
                kda: "1 / 8 / 2",
                game_time_seconds: 621.0,
            },
        );

        assert!(prompt.system.len() < 1500);
        assert!(prompt.user.len() < 100);
        assert!(prompt.system.contains("JSON only"));
        assert!(prompt.system.contains("protected-class hate"));
        assert!(prompt.system.contains("soul-destroying"));
        assert!(prompt.system.contains("FEEDING FRENZY"));
        assert!(prompt.user.contains("champion=Yasuo"));
        assert!(prompt.user.contains("killer=Annie Bot"));
        assert!(prompt.user.contains("time=10:21"));
    }
}
