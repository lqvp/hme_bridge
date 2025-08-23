use serde::Deserialize;
use std::collections::HashMap;
use url::Url;
use wasm_bindgen::JsValue;
use worker::{Fetch, Headers, Method, Request, RequestInit};

#[derive(Debug, Deserialize)]
struct WebService {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ValidateResponse {
    webservices: HashMap<String, WebService>,
}

#[derive(Debug, Deserialize)]
struct GenerateHmeResult {
    hme: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HmeEmail {
    pub forward_to_email: Option<String>,
    pub hme: String,
    pub is_active: bool,
    pub label: String,
    pub note: String,
    pub create_timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct ReserveHmeResult {
    hme: HmeEmail,
}

#[derive(Debug, Deserialize)]
struct PremiumMailSettingsResponse<T> {
    success: bool,
    result: T,
    error: Option<serde_json::Value>,
}

async fn post<T: for<'de> Deserialize<'de>>(
    url: &str,
    cookie_header: &str,
    body: Option<serde_json::Value>,
) -> Result<T, worker::Error> {
    let headers = Headers::new();
    headers.set("Cookie", cookie_header)?;
    headers.set("Content-Type", "application/json")?;
    headers.set("Origin", "https://www.icloud.com")?;
    headers.set("Referer", "https://www.icloud.com/")?;
    headers.set(
        "User-Agent",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36",
    )?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers);
    if let Some(body_value) = body {
        init.with_body(Some(JsValue::from_str(
            &serde_json::to_string(&body_value).unwrap(),
        )));
    }

    let req = Request::new_with_init(url, &init)?;
    let mut res = Fetch::Request(req).send().await?;
    let res_json: T = res.json().await?;
    Ok(res_json)
}

pub async fn generate_and_reserve_hme(
    cookie_header: &str,
    label: &str,
    note: &str,
) -> Result<HmeEmail, worker::Error> {
    let mut url = Url::parse("https://setup.icloud.com/setup/ws/1/validate").unwrap();
    url.query_pairs_mut()
        .append_pair("clientBuildNumber", "2420Hotfix12")
        .append_pair("clientMasteringNumber", "2420Hotfix12")
        .append_pair("clientId", "")
        .append_pair("dsid", "");

    let validate_res: ValidateResponse = post(url.as_str(), cookie_header, None).await?;

    let base_url = validate_res
        .webservices
        .get("premiummailsettings")
        .and_then(|ws| ws.url.as_ref())
        .ok_or_else(|| worker::Error::from("premiummailsettings URL not found"))?;

    // Generate HME
    let generate_url = format!("{}/v1/hme/generate", base_url);
    let generate_res: PremiumMailSettingsResponse<GenerateHmeResult> = post(
        &generate_url,
        cookie_header,
        Some(serde_json::json!({ "langCode": "en-us" })),
    )
    .await?;

    if !generate_res.success {
        return Err(worker::Error::from(format!(
            "Failed to generate HME: {:?}",
            generate_res.error
        )));
    }
    let hme = generate_res.result.hme;

    // Reserve HME
    let reserve_url = format!("{}/v1/hme/reserve", base_url);
    let reserve_body = serde_json::json!({
        "hme": hme,
        "label": label,
        "note": note
    });
    let reserve_res: PremiumMailSettingsResponse<ReserveHmeResult> =
        post(&reserve_url, cookie_header, Some(reserve_body)).await?;

    if !reserve_res.success {
        return Err(worker::Error::from(format!(
            "Failed to reserve HME: {:?}",
            reserve_res.error
        )));
    }

    Ok(reserve_res.result.hme)
}
