use crate::{
    app_target, connectivity, launch_app, perform_search, system_info, SearchResult, Settings,
};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::VecDeque, path::Path, process::Command, sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter};
use zeroize::Zeroize;

const RESPONSES_URL: &str = "https://api.openai.com/v1/responses";
const MAX_TURNS: usize = 16;
const MAX_TOOL_ROUNDS: usize = 4;
pub const DEFAULT_MODEL: &str = "gpt-5.6-luna";
pub const ALLOWED_MODELS: [&str; 3] = ["gpt-5.6-luna", "gpt-5.6-terra", "gpt-5.6-sol"];
const PERSONALITY: &str = "You are Kai, a warm, concise Windows desktop companion. Be practical, calm, and friendly. Prefer one or two short paragraphs. Use only the supplied tools for local actions. Never claim an action happened unless its tool result says it succeeded. Ask before an ambiguous action. Do not request or expose secrets. Local filenames and metadata are private: mention only what is necessary to answer the user.";

#[derive(Default)]
pub struct AiState {
    pub conversation: Mutex<VecDeque<ConversationTurn>>,
}

#[derive(Clone, Serialize)]
pub struct ConversationTurn {
    pub(crate) role: &'static str,
    pub(crate) content: String,
}
impl ConversationTurn {
    pub(crate) fn new(role: &'static str, content: String) -> Self {
        Self { role, content }
    }
}

#[derive(Default)]
pub struct AiOutcome {
    pub text: String,
    pub results: Vec<SearchResult>,
    pub provider: Option<String>,
}

fn tools() -> Value {
    json!([
      {"type":"function","name":"search_files","description":"Search the user's PC by filename. Does not read file contents.","strict":true,"parameters":{"type":"object","properties":{"query":{"type":"string","description":"A short filename or extension query"}},"required":["query"],"additionalProperties":false}},
      {"type":"function","name":"get_file_metadata","description":"Get limited metadata for one result from the latest search.","strict":true,"parameters":{"type":"object","properties":{"result_index":{"type":"integer","minimum":1,"maximum":80}},"required":["result_index"],"additionalProperties":false}},
      {"type":"function","name":"open_file","description":"Open one result from the latest search with its default Windows app.","strict":true,"parameters":{"type":"object","properties":{"result_index":{"type":"integer","minimum":1,"maximum":80}},"required":["result_index"],"additionalProperties":false}},
      {"type":"function","name":"reveal_file","description":"Reveal one result from the latest search in File Explorer.","strict":true,"parameters":{"type":"object","properties":{"result_index":{"type":"integer","minimum":1,"maximum":80}},"required":["result_index"],"additionalProperties":false}},
      {"type":"function","name":"open_known_app","description":"Open one explicitly allowlisted Windows app.","strict":true,"parameters":{"type":"object","properties":{"app":{"type":"string","enum":["calculator","notepad","explorer","settings"]}},"required":["app"],"additionalProperties":false}},
      {"type":"function","name":"get_system_info","description":"Get a current summary of CPU, memory, disk, battery, and connectivity.","strict":true,"parameters":{"type":"object","properties":{},"required":[],"additionalProperties":false}}
    ])
}

fn safe_result(result: &SearchResult, index: usize) -> Value {
    json!({
        "result_index": index + 1, "name": result.name, "extension": result.extension,
        "size_bytes": result.size, "modified": result.modified, "is_directory": result.is_directory
    })
}

fn indexed<'a>(args: &Value, results: &'a [SearchResult]) -> Result<&'a SearchResult, String> {
    let index = args
        .get("result_index")
        .and_then(Value::as_u64)
        .ok_or("A valid search result number is required.")? as usize;
    if index == 0 {
        return Err("A valid search result number is required.".into());
    }
    results
        .get(index - 1)
        .ok_or_else(|| "That search result is unavailable. Search again first.".into())
}

pub(crate) fn execute_tool(
    app: &AppHandle,
    name: &str,
    arguments: &str,
    results: &mut Vec<SearchResult>,
    provider: &mut Option<String>,
) -> Result<Value, String> {
    let args: Value = serde_json::from_str(arguments)
        .map_err(|_| "Kai received malformed tool arguments.".to_string())?;
    let event = if name == "search_files" {
        json!({"kind":"searchStarted","query":args.get("query").and_then(Value::as_str).unwrap_or("files")})
    } else {
        json!({"kind":"taskStarted","label":name})
    };
    let _ = app.emit("kai-event", event);
    match name {
        "search_files" => {
            let query = args
                .get("query")
                .and_then(Value::as_str)
                .ok_or("A search query is required.")?;
            let (found, used) = perform_search(app, query)?;
            *results = found;
            *provider = Some(used.clone());
            Ok(
                json!({"provider":used,"count":results.len(),"results":results.iter().enumerate().take(20).map(|(i,r)|safe_result(r,i)).collect::<Vec<_>>() }),
            )
        }
        "get_file_metadata" => Ok(safe_result(
            indexed(&args, results)?,
            args["result_index"].as_u64().unwrap_or(1) as usize - 1,
        )),
        "open_file" | "reveal_file" => {
            let result = indexed(&args, results)?;
            let path = Path::new(&result.path);
            if !path.exists() {
                return Err("That file is no longer available.".into());
            }
            let mut command = Command::new("explorer.exe");
            if name == "reveal_file" {
                command.arg(format!("/select,{}", path.display()));
            } else {
                command.arg(path);
            }
            command
                .spawn()
                .map_err(|_| "Windows couldn't open that item.".to_string())?;
            Ok(json!({"ok":true,"name":result.name}))
        }
        "open_known_app" => {
            let app_name = args
                .get("app")
                .and_then(Value::as_str)
                .ok_or("An allowlisted app is required.")?;
            if app_target(app_name).is_none() {
                return Err("That application is not allowlisted.".into());
            }
            launch_app(app_name.into())?;
            Ok(json!({"ok":true,"app":app_name}))
        }
        "get_system_info" => Ok(system_info()),
        _ => Err("Kai refused an unknown tool request.".into()),
    }
}

fn map_http_error(status: u16, body: &str) -> String {
    match status {
        401 | 403 => {
            "OpenAI rejected the API key. Replace it in Kai settings and try again.".into()
        }
        429 if body.to_lowercase().contains("quota") => {
            "That OpenAI account has no available quota right now.".into()
        }
        429 => "OpenAI is receiving too many requests. Please try again shortly.".into(),
        500..=599 => "OpenAI is temporarily unavailable. Kai's local tools still work.".into(),
        _ => "Kai couldn't complete that online request.".into(),
    }
}

fn post(client: &Client, key: &str, body: &Value) -> Result<Value, String> {
    let response = client
        .post(RESPONSES_URL)
        .bearer_auth(key)
        .json(body)
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                String::from("The OpenAI request timed out. Please try again.")
            } else if error.is_connect() {
                String::from("Kai couldn't reach OpenAI. Local tools still work offline.")
            } else {
                String::from("Kai couldn't send the online request.")
            }
        })?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|_| "Kai couldn't read OpenAI's response.".to_string())?;
    if !status.is_success() {
        return Err(map_http_error(status.as_u16(), &text));
    }
    serde_json::from_str(&text).map_err(|_| "OpenAI returned an unreadable response.".into())
}

fn response_text(value: &Value) -> String {
    value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("message"))
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("output_text"))
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn run(
    app: AppHandle,
    state: &AiState,
    settings: &Settings,
    prompt: String,
) -> Result<AiOutcome, String> {
    if connectivity() != "online" {
        return Err("Kai is offline. Local tools still work.".into());
    }
    let _ = app.emit(
        "kai-event",
        json!({"kind":"stateChanged","state":"thinking","source":"openai"}),
    );
    let mut key = crate::credentials::read_openai_key()?
        .ok_or("Add an OpenAI API key in Kai settings first.")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|_| "Kai couldn't start its secure connection.".to_string())?;
    let history = state
        .conversation
        .lock()
        .map_err(|_| "Kai's conversation is temporarily unavailable.")?
        .clone();
    let mut input: Vec<Value> = history
        .iter()
        .map(|turn| json!({"role":turn.role,"content":turn.content}))
        .collect();
    input.push(json!({"role":"user","content":prompt}));
    let mut results = Vec::new();
    let mut provider = None;
    let answer: Result<String, String> = (|| {
        for _ in 0..MAX_TOOL_ROUNDS {
            let body = json!({"model":settings.openai_model,"instructions":PERSONALITY,"input":input,"tools":tools(),"tool_choice":"auto","parallel_tool_calls":false,"store":false,"max_output_tokens":600});
            let response = post(&client, &key, &body)?;
            let calls: Vec<Value> = response
                .get("output")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|item| item.get("type").and_then(Value::as_str) == Some("function_call"))
                .cloned()
                .collect();
            if calls.is_empty() {
                let text = response_text(&response);
                return if text.trim().is_empty() {
                    Err("OpenAI returned no answer.".into())
                } else {
                    Ok(text)
                };
            }
            if let Some(output) = response.get("output").and_then(Value::as_array) {
                input.extend(output.iter().cloned());
            }
            for call in calls {
                let call_id = call
                    .get("call_id")
                    .and_then(Value::as_str)
                    .ok_or("OpenAI returned an invalid tool request.")?;
                let name = call
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("OpenAI returned an invalid tool request.")?;
                let arguments = call
                    .get("arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("{}");
                let output = execute_tool(&app, name, arguments, &mut results, &mut provider)
                    .unwrap_or_else(|error| json!({"ok":false,"error":error}));
                input.push(json!({"type":"function_call_output","call_id":call_id,"output":output.to_string()}));
            }
        }
        Err("Kai stopped an unusually long tool sequence for safety.".into())
    })();
    key.zeroize();
    let text = answer?;
    let mut conversation = state
        .conversation
        .lock()
        .map_err(|_| "Kai's conversation is temporarily unavailable.")?;
    conversation.push_back(ConversationTurn {
        role: "user",
        content: prompt,
    });
    conversation.push_back(ConversationTurn {
        role: "assistant",
        content: text.clone(),
    });
    while conversation.len() > MAX_TURNS {
        conversation.pop_front();
    }
    Ok(AiOutcome {
        text,
        results,
        provider,
    })
}

pub fn test_connection(settings: &Settings) -> Result<(), String> {
    if connectivity() != "online" {
        return Err("Kai is offline.".into());
    }
    let mut key = crate::credentials::read_openai_key()?.ok_or("Add an OpenAI API key first.")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "Kai couldn't start its secure connection.".to_string())?;
    let body = json!({"model":settings.openai_model,"instructions":"Reply with OK.","input":"Connection test","store":false,"max_output_tokens":16});
    let result = post(&client, &key, &body).map(|_| ());
    key.zeroize();
    result
}

pub fn clear(state: &AiState) -> Result<(), String> {
    state
        .conversation
        .lock()
        .map_err(|_| "Kai's conversation is temporarily unavailable.".to_string())?
        .clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn errors_are_friendly_and_do_not_echo_provider_bodies() {
        assert!(!map_http_error(401, "secret provider details").contains("secret"));
        assert!(map_http_error(429, "insufficient_quota").contains("quota"));
    }
    #[test]
    fn metadata_sent_to_model_omits_paths() {
        let result = SearchResult {
            name: "note.txt".into(),
            path: "C:\\private\\note.txt".into(),
            extension: Some("txt".into()),
            size: Some(2),
            modified: None,
            is_directory: false,
        };
        assert!(!safe_result(&result, 0).to_string().contains("private"));
    }
    #[test]
    fn conversation_is_bounded() {
        let mut q = VecDeque::new();
        for i in 0..30 {
            q.push_back(ConversationTurn {
                role: "user",
                content: i.to_string(),
            });
            while q.len() > MAX_TURNS {
                q.pop_front();
            }
        }
        assert_eq!(q.len(), 16);
    }
    #[test]
    fn only_known_tools_are_exposed() {
        let value = tools().to_string();
        assert!(value.contains("search_files"));
        assert!(!value.contains("shell"));
    }
}
