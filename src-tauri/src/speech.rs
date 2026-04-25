use crate::error::{AppError, AppResult};
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const GEMINI_TTS_MODEL: &str = "gemini-2.5-flash-preview-tts";
const GEMINI_TTS_VOICE: &str = "Fenrir";
const PCM_SAMPLE_RATE: u32 = 24_000;
const PCM_CHANNELS: u16 = 1;
const PCM_BITS_PER_SAMPLE: u16 = 16;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TtsRequest<'a> {
    contents: [TtsContent<'a>; 1],
    generation_config: TtsGenerationConfig<'a>,
}

#[derive(Debug, Serialize)]
struct TtsContent<'a> {
    parts: [TtsPart<'a>; 1],
}

#[derive(Debug, Serialize)]
struct TtsPart<'a> {
    text: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TtsGenerationConfig<'a> {
    response_modalities: [&'static str; 1],
    speech_config: TtsSpeechConfig<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TtsSpeechConfig<'a> {
    voice_config: TtsVoiceConfig<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TtsVoiceConfig<'a> {
    prebuilt_voice_config: TtsPrebuiltVoiceConfig<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TtsPrebuiltVoiceConfig<'a> {
    voice_name: &'a str,
}

#[derive(Debug, Deserialize)]
struct TtsResponse {
    candidates: Vec<TtsCandidate>,
}

#[derive(Debug, Deserialize)]
struct TtsCandidate {
    content: TtsResponseContent,
}

#[derive(Debug, Deserialize)]
struct TtsResponseContent {
    parts: Vec<TtsResponsePart>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TtsResponsePart {
    inline_data: Option<TtsInlineData>,
}

#[derive(Debug, Deserialize)]
struct TtsInlineData {
    data: String,
}

pub async fn synthesize_gemini_speech(api_key: &str, text: &str) -> AppResult<Vec<u8>> {
    let api_key = api_key.trim();
    let text = text.trim();

    if api_key.is_empty() {
        return Err(AppError::Ai("Gemini API key is not set.".to_string()));
    }

    if text.is_empty() {
        return Err(AppError::Audio("No speech text was provided.".to_string()));
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{GEMINI_TTS_MODEL}:generateContent"
    );
    let speech_text = format!("Say this with barely-contained rage and deep disappointment — like someone who has watched you int their promos for the third game in a row and can't believe you exist. Furious, seething, but also genuinely let down: {text}");
    let request = TtsRequest {
        contents: [TtsContent {
            parts: [TtsPart { text: &speech_text }],
        }],
        generation_config: TtsGenerationConfig {
            response_modalities: ["AUDIO"],
            speech_config: TtsSpeechConfig {
                voice_config: TtsVoiceConfig {
                    prebuilt_voice_config: TtsPrebuiltVoiceConfig {
                        voice_name: GEMINI_TTS_VOICE,
                    },
                },
            },
        },
    };

    let response = Client::new()
        .post(url)
        .header("x-goog-api-key", api_key)
        .json(&request)
        .send()
        .await
        .map_err(AppError::from)?;

    if !response.status().is_success() {
        return Err(AppError::Audio(format!(
            "Gemini speech synthesis returned {}",
            response.status()
        )));
    }

    let response = response
        .json::<TtsResponse>()
        .await
        .map_err(AppError::from)?;
    let base64_pcm = response
        .candidates
        .first()
        .and_then(|candidate| candidate.content.parts.first())
        .and_then(|part| part.inline_data.as_ref())
        .map(|inline_data| inline_data.data.as_str())
        .ok_or_else(|| AppError::Audio("Gemini returned no speech audio.".to_string()))?;

    let pcm = general_purpose::STANDARD
        .decode(base64_pcm)
        .map_err(|error| AppError::Audio(error.to_string()))?;

    Ok(pcm_to_wav(pcm))
}

fn pcm_to_wav(pcm: Vec<u8>) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let byte_rate = PCM_SAMPLE_RATE * PCM_CHANNELS as u32 * PCM_BITS_PER_SAMPLE as u32 / 8;
    let block_align = PCM_CHANNELS * PCM_BITS_PER_SAMPLE / 8;
    let mut wav = Vec::with_capacity(44 + pcm.len());

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&PCM_CHANNELS.to_le_bytes());
    wav.extend_from_slice(&PCM_SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&PCM_BITS_PER_SAMPLE.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&pcm);

    wav
}

#[cfg(test)]
mod tests {
    use super::pcm_to_wav;

    #[test]
    fn wraps_pcm_in_wav_header() {
        let wav = pcm_to_wav(vec![0, 1, 2, 3]);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4);
        assert_eq!(&wav[44..], &[0, 1, 2, 3]);
    }
}
