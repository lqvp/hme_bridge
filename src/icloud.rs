use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct WebService {
    url: Option<String>,
    // status is unused
    // status: Option<String>,
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
    // pub origin: String,
    // pub anonymous_id: String,
    // pub domain: String,
    pub forward_to_email: Option<String>,
    pub hme: String,
    pub is_active: bool,
    pub label: String,
    pub note: String,
    pub create_timestamp: i64,
    // pub recipient_mail_id: String,
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

// Client struct placeholder for future expansion

pub async fn generate_and_reserve_hme(
    cookie_header: &str,
    label: &str,
    note: &str,
) -> Result<HmeEmail, Box<dyn std::error::Error>> {
    // 1. Create a client with cookie support
    let client = reqwest::Client::builder().cookie_store(true).build()?;

    // 2. Validate token and get the webservice URL
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", cookie_header.parse()?);
    headers.insert("Content-Type", "application/json".parse()?);
    headers.insert("Origin", "https://www.icloud.com".parse()?);
    headers.insert("Referer", "https://www.icloud.com/".parse()?);
    headers.insert(
        "User-Agent",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36".parse()?,
    );

    let params = [
        ("clientBuildNumber", "2420Hotfix12"),
        ("clientMasteringNumber", "2420Hotfix12"),
        ("clientId", ""), // This might need a real value
        ("dsid", ""),     // This might need a real value
    ];

    let response = client
        .post("https://setup.icloud.com/setup/ws/1/validate")
        .headers(headers.clone())
        .query(&params)
        .send()
        .await?;

    let response_text = response.text().await?;
    tracing::debug!("Validate response body: {}", response_text);

    let validate_res: ValidateResponse = serde_json::from_str(&response_text)?;

    let base_url = &validate_res
        .webservices
        .get("premiummailsettings")
        .ok_or("premiummailsettings service not found")?
        .url
        .as_ref()
        .ok_or("premiummailsettings URL is null")?;

    // 3. Generate HME
    let generate_url = format!("{}/v1/hme/generate", base_url);
    let response = client
        .post(generate_url)
        .headers(headers.clone())
        .query(&params)
        .json(&serde_json::json!({ "langCode": "en-us" }))
        .send()
        .await?;

    let response_text = response.text().await?;
    tracing::debug!("Generate HME response body: {}", response_text);

    let generate_res: PremiumMailSettingsResponse<GenerateHmeResult> =
        serde_json::from_str(&response_text)?;

    if !generate_res.success {
        return Err(format!("Failed to generate HME: {:?}", generate_res.error).into());
    }
    let hme = generate_res.result.hme;

    // 4. Reserve HME
    let reserve_url = format!("{}/v1/hme/reserve", base_url);
    let reserve_body = serde_json::json!({
        "hme": hme,
        "label": label,
        "note": note
    });
    let response = client
        .post(reserve_url)
        .headers(headers)
        .query(&params)
        .json(&reserve_body)
        .send()
        .await?;

    let response_text = response.text().await?;
    tracing::debug!("Reserve HME response body: {}", response_text);

    let reserve_res: PremiumMailSettingsResponse<ReserveHmeResult> =
        serde_json::from_str(&response_text)?;

    if !reserve_res.success {
        return Err(format!("Failed to reserve HME: {:?}", reserve_res.error).into());
    }

    Ok(reserve_res.result.hme)
}
