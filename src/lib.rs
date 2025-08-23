use chrono::{TimeZone, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use worker::kv::KvStore;
use worker::*;

mod icloud;
use crate::icloud::HmeEmail;

const KV_KEY: &str = "credentials";

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Credential {
    label: String,
    token: String,
    cookie: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct CredentialRequest {
    label: String,
    cookie: serde_json::Value,
}

fn generate_token() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

async fn get_credentials(kv: &KvStore) -> Result<Vec<Credential>> {
    let creds_json = kv
        .get(KV_KEY)
        .text()
        .await?
        .unwrap_or_else(|| "[]".to_string());
    serde_json::from_str(&creds_json).map_err(|e| worker::Error::from(e.to_string()))
}

async fn save_credentials(kv: &KvStore, creds: &[Credential]) -> Result<()> {
    let json = serde_json::to_string(creds).map_err(|e| worker::Error::from(e.to_string()))?;
    kv.put(KV_KEY, json)?.execute().await.map_err(|e| e.into())
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().and_then(|cf| cf.coordinates()).unwrap_or_default(),
        req.cf()
            .and_then(|cf| cf.region())
            .unwrap_or_else(|| "unknown region".into())
    );
}

async fn admin_auth(req: &Request, ctx: &RouteContext<()>) -> Result<KvStore> {
    let admin_token = ctx.secret("ADMIN_TOKEN")?.to_string();
    let auth_header = req
        .headers()
        .get("x-admin-token")?
        .ok_or_else(|| worker::Error::from("X-Admin-Token header is missing"))?;

    if auth_header != admin_token {
        return Err(worker::Error::from("Unauthorized"));
    }

    ctx.kv("HME_BRIDGE_CREDS")
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);
    console_error_panic_hook::set_once();

    let router = Router::new()
        .get_async("/admin/credentials", |_req, ctx| async move {
            let kv = admin_auth(&_req, &ctx).await?;
            let creds = get_credentials(&kv).await?;
            Response::from_json(&creds)
        })
        .post_async("/admin/credentials", |mut req, ctx| async move {
            let kv = admin_auth(&req, &ctx).await?;
            let mut creds = get_credentials(&kv).await?;

            let new_req: CredentialRequest = req.json().await?;

            let new_cred = Credential {
                label: new_req.label,
                cookie: serde_json::to_string(&new_req.cookie)?,
                token: generate_token(),
            };
            creds.push(new_cred.clone());

            save_credentials(&kv, &creds).await?;

            Response::from_json(&new_cred)
        })
        .put_async("/admin/credentials/:token", |mut req, ctx| async move {
            let kv = admin_auth(&req, &ctx).await?;
            let token_to_update = ctx.param("token").unwrap();
            let mut creds = get_credentials(&kv).await?;

            let updated_req: CredentialRequest = req.json().await?;

            if let Some(cred) = creds.iter_mut().find(|c| c.token == *token_to_update) {
                cred.label = updated_req.label;
                cred.cookie = serde_json::to_string(&updated_req.cookie)?;
                let cred_clone = cred.clone();
                save_credentials(&kv, &creds).await?;
                Response::from_json(&cred_clone)
            } else {
                Response::error("Token not found", 404)
            }
        })
        .delete_async("/admin/credentials/:token", |req, ctx| async move {
            let kv = admin_auth(&req, &ctx).await?;
            let token_to_delete = ctx.param("token").unwrap();
            let mut creds = get_credentials(&kv).await?;

            let original_len = creds.len();
            creds.retain(|c| c.token != *token_to_delete);

            if creds.len() < original_len {
                save_credentials(&kv, &creds).await?;
                Response::ok("Credential deleted")
            } else {
                Response::error("Token not found", 404)
            }
        })
        .post_async("/api/alias/random/new", |mut req, ctx| async move {
            let keys_to_use = &[
                "X-APPLE-DS-WEB-SESSION-TOKEN",
                "X-APPLE-WEBAUTH-TOKEN",
                "X-APPLE-WEBAUTH-USER",
            ];

            let cookie_header = 'auth: {
                // 1. Try Authorization: Bearer token header first
                if let Ok(Some(auth_header)) = req.headers().get("authorization") {
                    if let Some(token) = auth_header.strip_prefix("Bearer ") {
                        let kv = ctx.kv("HME_BRIDGE_CREDS")?;
                        if let Ok(Some(creds_json)) = kv.get(KV_KEY).text().await {
                            if let Ok(creds) = serde_json::from_str::<Vec<Credential>>(&creds_json)
                            {
                                if let Some(cred) = creds.iter().find(|c| c.token == token) {
                                    if let Ok(header) =
                                        parse_cookies_from_json(&cred.cookie, keys_to_use)
                                    {
                                        break 'auth Ok(header);
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Try `authentication` header (for Bitwarden)
                if let Ok(Some(api_key)) = req.headers().get("authentication") {
                    if api_key.is_empty() {
                        break 'auth Err(Response::error("Authentication header is empty", 401));
                    }

                    // Try parsing as JSON cookie first
                    if let Ok(header) = parse_cookies_from_json(&api_key, keys_to_use) {
                        break 'auth Ok(header);
                    }

                    // If not JSON, treat as a token and look up in KV
                    let token = api_key;
                    let kv = match ctx.kv("HME_BRIDGE_CREDS") {
                        Ok(kv) => kv,
                        Err(_) => {
                            break 'auth Err(Response::error("Internal Server Error", 500));
                        }
                    };

                    let credentials_json = match kv.get(KV_KEY).text().await {
                        Ok(Some(json)) => json,
                        _ => {
                            break 'auth Err(Response::error("Invalid credentials", 401));
                        }
                    };

                    let credentials: Vec<Credential> = match serde_json::from_str(&credentials_json)
                    {
                        Ok(creds) => creds,
                        Err(_) => {
                            break 'auth Err(Response::error("Invalid credentials format", 500));
                        }
                    };

                    if let Some(cred) = credentials.iter().find(|c| c.token == token) {
                        if let Ok(header) = parse_cookies_from_json(&cred.cookie, keys_to_use) {
                            break 'auth Ok(header);
                        }
                    }
                }

                break 'auth Err(Response::error(
                    "Authentication header is missing or invalid",
                    401,
                ));
            };

            let cookie_header = match cookie_header {
                Ok(h) => h,
                Err(e) => return e,
            };

            if cookie_header.is_empty() {
                return Response::error("Required cookies not found in stored credential", 500);
            }

            let payload: CreateAliasRequest = match req.json().await {
                Ok(p) => p,
                Err(_) => return Response::error("Bad request", 400),
            };

            let note = payload
                .note
                .unwrap_or_else(|| "Generated by Bitwarden.".to_string());
            let label = "Generated by hme_bridge";

            match icloud::generate_and_reserve_hme(&cookie_header, label, &note).await {
                Ok(hme_email) => {
                    let alias = hme_to_alias(hme_email);
                    Response::from_json(&alias)
                }
                Err(e) => {
                    console_error!("Failed to generate HME: {}", e);
                    Response::error(format!("Internal Server Error: {}", e), 500)
                }
            }
        });

    router.run(req, env).await
}

#[derive(Deserialize, Serialize)]
struct CookieObject {
    name: String,
    value: String,
}

fn parse_cookies_from_json(
    json_str: &str,
    keys: &[&str],
) -> std::result::Result<String, serde_json::Error> {
    let cookies: Vec<CookieObject> = serde_json::from_str(json_str)?;
    let cookie_header = cookies
        .into_iter()
        .filter(|c| keys.contains(&c.name.as_str()))
        .map(|c| format!("{}={}", c.name, c.value))
        .collect::<Vec<String>>()
        .join("; ");
    Ok(cookie_header)
}

fn hme_to_alias(hme: HmeEmail) -> Alias {
    let mailbox = Mailbox {
        id: 1, // dummy id
        email: hme
            .forward_to_email
            .unwrap_or_else(|| "forwarding-not-set@icloud.com".to_string()),
    };
    Alias {
        id: 1, // dummy id
        alias: hme.hme,
        name: Some(hme.label),
        enabled: hme.is_active,
        creation_timestamp: hme.create_timestamp,
        creation_date: Utc
            .timestamp_millis_opt(hme.create_timestamp)
            .unwrap()
            .to_rfc3339(),
        note: Some(hme.note),
        nb_block: 0,
        nb_forward: 0,
        nb_reply: 0,
        support_pgp: false,
        disable_pgp: false,
        mailbox: mailbox.clone(),
        mailboxes: vec![mailbox],
        latest_activity: None,
        pinned: false,
    }
}

#[derive(Debug, Deserialize)]
struct CreateAliasRequest {
    note: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Alias {
    id: i32,
    alias: String,
    name: Option<String>,
    enabled: bool,
    creation_timestamp: i64,
    creation_date: String,
    note: Option<String>,
    nb_block: i32,
    nb_forward: i32,
    nb_reply: i32,
    support_pgp: bool,
    disable_pgp: bool,
    mailbox: Mailbox,
    mailboxes: Vec<Mailbox>,
    latest_activity: Option<Activity>,
    pinned: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Mailbox {
    id: i32,
    email: String,
}

#[derive(Serialize, Deserialize)]
struct Activity {
    action: String,
    timestamp: i64,
    contact: Contact,
}

#[derive(Serialize, Deserialize)]
struct Contact {
    email: String,
    name: Option<String>,
    reverse_alias: String,
}
