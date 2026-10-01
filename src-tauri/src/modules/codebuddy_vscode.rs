//! CodeBuddy VSCode 插件版平台支持。
//!
//! 与 `codebuddy_cn`（CodeBuddy 国内版桌面 IDE）共用同一份账号池（登录 / 刷新走
//! codebuddy.cn），区别在于登录态落点：
//! 桌面 IDE 用 `<data>\CodeBuddy CN\User\globalStorage\state.vscdb` +
//! Keychain "CodeBuddy CN Safe Storage"；VS Code 插件版用
//! `%APPDATA%\Code\User\globalStorage\state.vscdb` + Keychain "Code Safe Storage"，
//! 且 secret 条目 key 也不同（`Tencent-Cloud.coding-copilot.new.accessToken`），
//! 因此单独成模块，只在「读写 VS Code 登录态」上做定制。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::codebuddy::{CodebuddyAccount, CodebuddyOAuthCompletePayload};

/// VS Code 里 CodeBuddy 扩展登录态条目的模糊匹配模式（SQLite LIKE，大小写不敏感）。
const VSCODE_SECRET_KEY_PATTERNS: [&str; 3] = [
    "secret://%coding-copilot%accessToken%",
    "secret://%planning-genie%accessToken%",
    "secret://%coding-copilot%token%",
];

/// 优先选用的条目 key（越靠前越优先）：第一项对应 VS Code 扩展自身写入的会话。
const VSCODE_SECRET_KEY_PREFERENCES: [&str; 2] = [
    "Tencent-Cloud.coding-copilot.new.accessToken",
    "planning-genie.new.accessToken",
];

/// VS Code 环境探测结果（用于前端诊断与提示）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebuddyVscodeEnv {
    /// VS Code 用户数据目录
    pub data_root: String,
    /// state.vscdb 路径
    pub state_db_path: String,
    /// state.vscdb 是否存在
    pub state_db_exists: bool,
    /// 是否找到了 CodeBuddy 扩展的登录态条目
    pub secret_found: bool,
    /// 实际命中的条目 key
    pub secret_item_key: Option<String>,
}

pub fn vscode_data_root() -> Result<PathBuf, String> {
    crate::modules::vscode_paths::resolve_vscode_data_root_for_state_db()
}

pub fn state_db_path(data_root: &Path) -> PathBuf {
    crate::modules::vscode_paths::vscode_state_db_path(data_root)
}

fn find_secret_item_key(data_root: &Path) -> Result<Option<String>, String> {
    crate::modules::vscode_inject::find_codebuddy_secret_item_key(
        data_root,
        &VSCODE_SECRET_KEY_PATTERNS,
        &VSCODE_SECRET_KEY_PREFERENCES,
    )
}

pub fn env_status() -> Result<CodebuddyVscodeEnv, String> {
    let data_root = vscode_data_root()?;
    let db_path = state_db_path(&data_root);
    let item_key = find_secret_item_key(&data_root).ok().flatten();
    Ok(CodebuddyVscodeEnv {
        data_root: data_root.to_string_lossy().to_string(),
        state_db_path: db_path.to_string_lossy().to_string(),
        state_db_exists: db_path.exists(),
        secret_found: item_key.is_some(),
        secret_item_key: item_key,
    })
}

/// 读取 VS Code 里当前 CodeBuddy 插件的登录态明文（会话 JSON）。
pub fn read_current_session() -> Result<Option<String>, String> {
    let data_root = vscode_data_root()?;
    let Some(item_key) = find_secret_item_key(&data_root)? else {
        return Ok(None);
    };
    crate::modules::vscode_inject::read_vscode_secret_value_by_item_key(&data_root, &item_key)
}

/// 从 VS Code 登录态里解析出 access token（剥离 `uid+token` 前缀）。
fn extract_access_token_from_session(secret: &str) -> Option<String> {
    let parsed_json = serde_json::from_str::<serde_json::Value>(secret).ok();
    let token_candidate = parsed_json
        .as_ref()
        .and_then(crate::modules::codebuddy_account::parse_local_access_token)
        .or_else(|| {
            let raw = secret.trim();
            if raw.is_empty() {
                None
            } else {
                Some(raw.to_string())
            }
        })?;

    let (_, suffix) =
        crate::modules::codebuddy_account::extract_local_codebuddy_token_parts(&token_candidate)?;
    crate::modules::codebuddy_account::normalize_local_codebuddy_token(&suffix)
}

/// 从 VS Code 登录态构造导入用的 payload（未落库）。
pub fn import_payload_from_local() -> Result<Option<CodebuddyOAuthCompletePayload>, String> {
    let Some(secret) = read_current_session()? else {
        return Ok(None);
    };
    if secret.trim().is_empty() {
        return Ok(None);
    }

    let parsed_json = serde_json::from_str::<serde_json::Value>(&secret).ok();
    let token_candidate = parsed_json
        .as_ref()
        .and_then(crate::modules::codebuddy_account::parse_local_access_token)
        .or_else(|| {
            let raw = secret.trim();
            if raw.is_empty() {
                None
            } else {
                Some(raw.to_string())
            }
        });

    let Some(raw_token) = token_candidate else {
        return Err("VS Code CodeBuddy 登录信息解析失败：未找到 access token".to_string());
    };

    let Some((uid_from_token, normalized_token)) =
        crate::modules::codebuddy_account::extract_local_codebuddy_token_parts(&raw_token)
    else {
        return Err("VS Code CodeBuddy 登录信息解析失败：access token 无效".to_string());
    };
    let Some(access_token) =
        crate::modules::codebuddy_account::normalize_local_codebuddy_token(&normalized_token)
    else {
        return Err("VS Code CodeBuddy 登录信息解析失败：access token 为空".to_string());
    };

    Ok(Some(
        crate::modules::codebuddy_account::build_local_import_payload(
            access_token,
            parsed_json,
            uid_from_token,
        ),
    ))
}

/// 生成写入 VS Code 的会话 JSON。
///
/// 优先在「VS Code 里已有的会话」基础上改写，只替换与凭据相关的字段：
/// 这样扩展自己写入的其它字段（`id`、`account` 结构、未知字段）都能保留，
/// 避免因为字段形状不一致导致扩展认为未登录；没有既有会话时才回落到
/// 桌面 IDE 同款的完整会话结构。
fn build_vscode_session_json(account: &CodebuddyAccount, existing: Option<&str>) -> String {
    let mut session: serde_json::Value = existing
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| {
            serde_json::from_str::<serde_json::Value>(
                &crate::modules::codebuddy_account::build_default_client_session_json(account),
            )
            .unwrap_or_else(|_| serde_json::json!({}))
        });

    let uid = account.uid.as_deref().unwrap_or("");
    let nickname = account.nickname.as_deref().unwrap_or("");
    let enterprise_id = account.enterprise_id.as_deref().unwrap_or("");
    let enterprise_name = account.enterprise_name.as_deref().unwrap_or("");
    let domain = account.domain.as_deref().unwrap_or("");
    let refresh_token = account.refresh_token.as_deref().unwrap_or("");
    let expires_at = account.expires_at.unwrap_or(0);
    let now_ms = chrono::Utc::now().timestamp_millis();

    if let Some(object) = session.as_object_mut() {
        object.insert("token".to_string(), serde_json::json!(account.access_token));
        object.insert("refreshToken".to_string(), serde_json::json!(refresh_token));
        object.insert("expiresAt".to_string(), serde_json::json!(expires_at));
        object.insert("domain".to_string(), serde_json::json!(domain));
        object.insert(
            "accessToken".to_string(),
            serde_json::json!(format!("{}+{}", uid, account.access_token)),
        );

        let auth = object
            .entry("auth".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(auth_object) = auth.as_object_mut() {
            auth_object.insert(
                "accessToken".to_string(),
                serde_json::json!(account.access_token),
            );
            auth_object.insert("refreshToken".to_string(), serde_json::json!(refresh_token));
            auth_object.insert(
                "tokenType".to_string(),
                serde_json::json!(account.token_type.as_deref().unwrap_or("Bearer")),
            );
            auth_object.insert("domain".to_string(), serde_json::json!(domain));
            auth_object.insert("expiresAt".to_string(), serde_json::json!(expires_at));
            auth_object.insert("expiresIn".to_string(), serde_json::json!(expires_at));
            auth_object.insert("lastRefreshTime".to_string(), serde_json::json!(now_ms));
        }

        let account_entry = object
            .entry("account".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if let Some(account_object) = account_entry.as_object_mut() {
            account_object.insert("id".to_string(), serde_json::json!(uid));
            account_object.insert("uid".to_string(), serde_json::json!(uid));
            account_object.insert("label".to_string(), serde_json::json!(nickname));
            account_object.insert("nickname".to_string(), serde_json::json!(nickname));
            account_object.insert(
                "enterpriseId".to_string(),
                serde_json::json!(enterprise_id),
            );
            account_object.insert(
                "enterpriseName".to_string(),
                serde_json::json!(enterprise_name),
            );
            account_object.insert("lastLogin".to_string(), serde_json::json!(true));
        }
    }

    session.to_string()
}

/// 把指定账号写入 VS Code 里 CodeBuddy 扩展的登录态。
///
/// 返回被切换账号的邮箱，便于前端直接展示提示。
pub fn inject_account_to_vscode(account_id: &str) -> Result<String, String> {
    let account = crate::modules::codebuddy_cn_account::load_account(account_id)
        .ok_or_else(|| format!("CodeBuddy 账号不存在: {}", account_id))?;

    let data_root = vscode_data_root()?;
    let db_path = state_db_path(&data_root);
    if !db_path.exists() {
        return Err(format!(
            "未找到 VS Code 的 state.vscdb：{}\n\n请确认本机已安装 VS Code，并在其中登录过一次 CodeBuddy 扩展。",
            db_path.display()
        ));
    }

    // 必须已存在扩展写入过的登录态条目：只有这样我们才能拿到 VS Code 真实使用的
    // secret 条目 key（各宿主 / 版本的大小写与命名并不一致），并据此原样回写。
    let Some(existing_key) = find_secret_item_key(&data_root)? else {
        return Err(format!(
            "未在 VS Code 中找到 CodeBuddy 扩展的登录态条目：{}\n\n\
             请先在 VS Code 中安装并登录一次 CodeBuddy 扩展，再使用切换功能。",
            state_db_path(&data_root).display()
        ));
    };
    let item_key = existing_key.clone();

    // 解密失败（例如 macOS 钥匙串被拒）时不阻断：仍然用完整会话结构写入。
    let existing_session = crate::modules::vscode_inject::read_vscode_secret_value_by_item_key(
        &data_root,
        &existing_key,
    )
    .ok()
    .flatten();

    let session_json = build_vscode_session_json(&account, existing_session.as_deref());
    crate::modules::vscode_inject::inject_secret_to_state_db_for_vscode(
        &db_path,
        &item_key,
        &session_json,
    )
    .map_err(|err| {
        format!(
            "写入 VS Code 登录态失败：{}\n\n可能原因：VS Code 从未登录过 CodeBuddy 扩展。\n\
             请先打开 VS Code 并登录一次 CodeBuddy 扩展，再使用切换功能。",
            err
        )
    })?;

    crate::modules::logger::log_info(&format!(
        "CodeBuddy VSCode 账号注入完成: email={}, db={}, key={}",
        account.email,
        db_path.to_string_lossy(),
        item_key
    ));

    Ok(account.email)
}

/// 在给定账号池中匹配出 VS Code 当前登录的账号 ID。
pub fn resolve_current_account_id(accounts: &[CodebuddyAccount]) -> Result<Option<String>, String> {
    let Some(secret) = read_current_session()? else {
        return Ok(None);
    };
    let Some(token) = extract_access_token_from_session(&secret) else {
        return Ok(None);
    };
    Ok(accounts
        .iter()
        .find(|account| account.access_token == token)
        .map(|account| account.id.clone()))
}
