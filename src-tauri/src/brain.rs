use crate::{
    ai::{AiOutcome, AiState, ConversationTurn},
    credentials,
};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{fs, sync::MutexGuard, time::Duration};
use tauri::{AppHandle, Manager};
use zeroize::Zeroize;

const MAX_TOOL_ROUNDS: usize = 4;
fn service_url() -> Option<String> {
    std::env::var("KAI_BRAIN_URL")
        .ok()
        .or_else(|| option_env!("KAI_BRAIN_URL").map(str::to_string))
        .map(|v| v.trim_end_matches('/').to_string())
        .filter(|v| {
            v.starts_with("https://") || cfg!(debug_assertions) && v.starts_with("http://127.0.0.1")
        })
}
fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|_| "Kai couldn't start a secure Brain connection.".into())
}
fn device_id(app: &AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("brain-device-id");
    if let Ok(value) = fs::read_to_string(&path) {
        let v = value.trim();
        if uuid::Uuid::parse_str(v).is_ok() {
            return Ok(v.into());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?
    }
    let value = uuid::Uuid::new_v4().to_string();
    fs::write(path, &value).map_err(|e| e.to_string())?;
    Ok(value)
}
fn enroll(app: &AppHandle, url: &str, client: &Client) -> Result<String, String> {
    let id = device_id(app)?;
    let response = client
        .post(format!("{url}/v1/devices/register"))
        .json(&json!({"deviceId":id}))
        .send()
        .map_err(|_| "Kai Brain is temporarily unavailable.".to_string())?;
    if !response.status().is_success() {
        return Err(if response.status().as_u16() == 403 {
            "Kai Brain registration is closed for this deployment.".into()
        } else {
            "Kai Brain registration failed.".into()
        });
    }
    let value: Value = response
        .json()
        .map_err(|_| "Kai Brain returned an invalid registration response.".to_string())?;
    let token = value
        .get("deviceToken")
        .and_then(Value::as_str)
        .ok_or("Kai Brain returned no device token.")?
        .to_string();
    credentials::write_brain_token(token.clone())?;
    Ok(token)
}
#[derive(Deserialize)]
struct BrainTool {
    name: String,
    args: Value,
}
#[derive(Deserialize)]
struct BrainResponse {
    kind: String,
    text: Option<String>,
    tool: Option<BrainTool>,
}
fn authorized_post(app:&AppHandle,path:&str,body:&Value)->Result<Value,String>{
    let url=service_url().ok_or("Kai Brain service URL is not configured in this build.")?;let client=client()?;
    let mut token=match credentials::read_brain_token()?{Some(value)=>value,None=>enroll(app,&url,&client)?};
    let send=|token:&str|client.post(format!("{url}{path}")).bearer_auth(token).json(body).send().map_err(|_|"Kai Brain is temporarily unavailable.".to_string());
    let mut response=send(&token)?;if response.status().as_u16()==401{credentials::delete_brain_token().ok();token=enroll(app,&url,&client)?;response=send(&token)?}
    token.zeroize();match response.status().as_u16(){200=>response.json().map_err(|_|"Kai Brain returned an invalid media response.".into()),429=>Err("Kai Brain is busy right now. Please try again shortly.".into()),_=>Err("Kai Brain's vision or speech provider is unavailable.".into())}
}
pub fn vision(app:&AppHandle,prompt:String,image:String)->Result<String,String>{let value=authorized_post(app,"/v1/vision",&json!({"prompt":prompt,"image":image}))?;value.get("text").and_then(Value::as_str).filter(|v|!v.trim().is_empty()).map(str::to_string).ok_or("Kai Brain returned no screen description.".into())}
pub fn transcribe(app:&AppHandle,audio:String,mime_type:String)->Result<String,String>{let value=authorized_post(app,"/v1/transcribe",&json!({"audio":audio,"mimeType":mime_type}))?;value.get("text").and_then(Value::as_str).filter(|v|!v.trim().is_empty()).map(str::to_string).ok_or("Kai Brain returned no transcription.".into())}
fn post(client: &Client, url: &str, token: &str, body: &Value) -> Result<BrainResponse, String> {
    let response = client
        .post(format!("{url}/v1/respond"))
        .bearer_auth(token)
        .json(body)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                String::from("Kai Brain took too long to respond.")
            } else {
                String::from("Kai Brain is temporarily unavailable. Local tools still work.")
            }
        })?;
    match response.status().as_u16() {
        200 => response
            .json()
            .map_err(|_| "Kai Brain returned an invalid response.".into()),
        401 => Err("Kai Brain device authorization expired.".into()),
        429 => Err("Kai Brain is busy right now. Please try again shortly.".into()),
        _ => Err("Kai Brain is temporarily unavailable. Local tools still work.".into()),
    }
}
fn history(
    state: &AiState,
) -> Result<
    (
        MutexGuard<'_, std::collections::VecDeque<ConversationTurn>>,
        Vec<Value>,
    ),
    String,
> {
    let guard = state
        .conversation
        .lock()
        .map_err(|_| "Kai's conversation is temporarily unavailable.".to_string())?;
    let messages = guard
        .iter()
        .map(|t| json!({"role":t.role,"content":t.content}))
        .collect();
    Ok((guard, messages))
}
pub fn run(app: AppHandle, state: &AiState, prompt: String) -> Result<AiOutcome, String> {
    let url = service_url().ok_or("Kai Brain service URL is not configured in this build.")?;
    let client = client()?;
    let mut token = match credentials::read_brain_token()? {
        Some(value) => value,
        None => enroll(&app, &url, &client)?,
    };
    let (guard, mut messages) = history(state)?;
    drop(guard);
    messages.push(json!({"role":"user","content":prompt}));
    let mut results = Vec::new();
    let mut provider = None;
    let mut tool_result: Option<Value> = None;
    for _ in 0..MAX_TOOL_ROUNDS {
        let mut body = json!({"messages":messages});
        if let Some(value) = tool_result.take() {
            body["toolResult"] = value;
        }
        let response = match post(&client, &url, &token, &body) {
            Ok(v) => v,
            Err(e) if e.contains("authorization expired") => {
                credentials::delete_brain_token().ok();
                token = enroll(&app, &url, &client)?;
                post(&client, &url, &token, &body)?
            }
            Err(e) => return Err(e),
        };
        match response.kind.as_str() {
            "answer" => {
                let text = response
                    .text
                    .filter(|v| !v.trim().is_empty())
                    .ok_or("Kai Brain returned no answer.")?;
                let (mut conversation, _) = history(state)?;
                conversation.push_back(ConversationTurn::new("user", prompt));
                conversation.push_back(ConversationTurn::new("assistant", text.clone()));
                while conversation.len() > 16 {
                    conversation.pop_front();
                }
                token.zeroize();
                return Ok(AiOutcome {
                    text,
                    results,
                    provider,
                });
            }
            "tool_request" => {
                let tool = response
                    .tool
                    .ok_or("Kai Brain returned an invalid tool request.")?;
                let args = tool.args.to_string();
                let output =
                    crate::ai::execute_tool(&app, &tool.name, &args, &mut results, &mut provider)
                        .unwrap_or_else(|error| json!({"ok":false,"error":error}));
                tool_result = Some(json!({"name":tool.name,"output":output}));
            }
            _ => return Err("Kai Brain returned an invalid response type.".into()),
        }
    }
    Err("Kai stopped an unusually long Brain tool sequence for safety.".into())
}
pub fn status() -> Value {
    let Some(url) = service_url() else {
        return json!({"configured":false,"connected":false,"mode":"local"});
    };
    let connected = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .ok()
        .and_then(|client| client.get(format!("{url}/healthz")).send().ok())
        .map(|response| response.status().is_success())
        .unwrap_or(false);
    json!({"configured":true,"connected":connected,"mode":if connected{"automatic"}else{"unavailable"}})
}
