use std::collections::BTreeMap;

use small_fixed_array::{FixedString, TruncatingInto as _};

use poise::serenity_prelude as serenity;

use tts_core::{
    opt_ext::OptionTryUnwrap as _,
    structs::{GoogleGender, GoogleVoice, Result, TTSMode, WebhookConfig, WebhookConfigRaw},
};

pub async fn get_webhooks(
    http: &serenity::Http,
    webhooks_raw: WebhookConfigRaw,
) -> Result<WebhookConfig> {
    let get_webhook = |url: reqwest::Url| async move {
        let (webhook_id, _) = serenity::parse_webhook(&url).try_unwrap()?;
        anyhow::Ok(webhook_id.to_webhook(http).await?)
    };

    let (logs, errors, dm_logs) = tokio::try_join!(
        get_webhook(webhooks_raw.logs),
        get_webhook(webhooks_raw.errors),
        get_webhook(webhooks_raw.dm_logs),
    )?;

    println!("Fetched webhooks");
    Ok(WebhookConfig {
        logs,
        errors,
        dm_logs,
    })
}

async fn fetch_json<T>(reqwest: &reqwest::Client, url: reqwest::Url, auth_header: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let resp = reqwest
        .get(url)
        .header("Authorization", auth_header)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(resp)
}

pub async fn fetch_voices<T: serde::de::DeserializeOwned>(
    reqwest: &reqwest::Client,
    mut tts_service: reqwest::Url,
    auth_key: Option<&str>,
    mode: TTSMode,
) -> Result<T> {
    tts_service.set_path("voices");
    tts_service
        .query_pairs_mut()
        .append_pair("mode", mode.into())
        .append_pair("raw", "true")
        .finish();

    let res = fetch_json(reqwest, tts_service, auth_key.unwrap_or("")).await?;

    println!("Loaded voices for TTS Mode: {mode}");
    Ok(res)
}

pub async fn fetch_translation_languages(
    reqwest: &reqwest::Client,
    mut tts_service: reqwest::Url,
    auth_key: Option<&str>,
) -> Result<BTreeMap<FixedString<u8>, FixedString<u8>>> {
    tts_service.set_path("translation_languages");

    let raw_langs: Vec<(String, FixedString<u8>)> =
        fetch_json(reqwest, tts_service, auth_key.unwrap_or("")).await?;

    let lang_map = raw_langs.into_iter().map(|(mut lang, name)| {
        lang.make_ascii_lowercase();
        (lang.trunc_into(), name)
    });

    println!("Loaded DeepL translation languages");
    Ok(lang_map.collect())
}

// pub fn prepare_gcloud_voices(
//     raw_map: Vec<GoogleVoice>,
// ) -> BTreeMap<FixedString<u8>, BTreeMap<FixedString<u8>, GoogleGender>> {
//     // {lang_accent: {variant: gender}}
//     let mut cleaned_map = BTreeMap::new();
//     for gvoice in raw_map {
//         let variant = gvoice
//             .name
//             .splitn(3, '-')
//             .nth(2)
//             .and_then(|mode_variant| mode_variant.split_once('-'))
//             .filter(|(mode, _)| *mode == "Standard")
//             .map(|(_, variant)| variant);

//         if let Some(variant) = variant {
//             let [language] = gvoice.language_codes;
//             cleaned_map
//                 .entry(language)
//                 .or_insert_with(BTreeMap::new)
//                 .insert(FixedString::from_str_trunc(variant), gvoice.ssml_gender);
//         }
//     }

//     cleaned_map
// }

pub fn prepare_gcloud_voices(
    raw_map: Vec<GoogleVoice>,
) -> BTreeMap<FixedString<u8>, BTreeMap<FixedString<u8>, GoogleGender>> {
    // {lang_accent: {variant: gender}}
    let mut cleaned_map = BTreeMap::new();
    for gvoice in raw_map {
        // 1. 先取得語言碼之後的完整部分 (例如 "Standard-A" 或 "Wavenet-F")
        if let Some(type_and_variant) = gvoice.name.splitn(3, '-').nth(2) {
            
            // 2. 判斷這部分是否為 "Standard-" 開頭
            let final_variant =
                if let Some(("Standard", variant_code)) = type_and_variant.split_once('-') {
                    // 是 Standard，使用舊格式 -> "A"
                    variant_code
                } else {
                    // 不是 Standard，使用新格式 -> "Wavenet-F"
                    type_and_variant
                };

            // 3. 使用正確的 snake_case 欄位名稱來存入 map
            let language = &gvoice.language_codes[0]; // 使用 language_codes
            cleaned_map
                .entry(FixedString::from_str_trunc(language))
                .or_insert_with(BTreeMap::new)
                .insert(FixedString::from_str_trunc(final_variant), gvoice.ssml_gender); // 使用 ssml_gender
        }
    }

    cleaned_map
}

pub async fn send_startup_message(
    http: &serenity::Http,
    log_webhook: &serenity::Webhook,
) -> Result<serenity::MessageId> {
    let startup_builder = serenity::ExecuteWebhook::default().content("**TTS Bot is starting up**");
    let startup_message = log_webhook.execute(http, true, startup_builder).await?;

    Ok(startup_message.unwrap().id)
}
