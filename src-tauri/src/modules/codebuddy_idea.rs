//! CodeBuddy IntelliJ IDEA 插件版平台支持。
//!
//! 与 `codebuddy_cn`（CodeBuddy 国内版桌面 IDE）共用同一份账号池（登录 / 刷新走
//! codebuddy.cn），区别在于登录态落点：
//!
//! IntelliJ 系 IDE 中的 CodeBuddy 插件（`coding-copilot-jetbrains`）把 VS Code 的
//! SecretStorage 模拟成 IntelliJ 的 `PersistentStateComponent`，落盘为**纯 XML**（未加密）：
//!
//! ```xml
//! <component name="SecretStorage">
//!   <Scores>
//!     <Entry key="Tencent-Cloud.coding-copilot.new.accessToken" value="{JSON}" />
//!   </Scores>
//! </component>
//! ```
//!
//! 文件位置：`<IDE 配置目录>/options/secret-storage.xml`。Windows 下配置目录为
//! `%APPDATA%\JetBrains\<IDE><版本>`，macOS 为 `~/Library/Application Support/JetBrains/<...>`，
//! Linux 为 `~/.config/JetBrains/<...>`。value 为会话 JSON，取其中 `auth.accessToken`
//! 即 JWT。因此单独成模块，只在「读写 IDEA 登录态」上做定制。

use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::models::codebuddy::{CodebuddyAccount, CodebuddyOAuthCompletePayload};

/// IDEA 里 CodeBuddy 插件登录态条目的候选 key（按优先级）。
///
/// 同一份 SecretStorage 里通常只有一个存在，与 `idea-dev-helper` 参考实现保持一致。
const IDEA_SECRET_KEY_PREFERENCES: [&str; 4] = [
    "Tencent-Cloud.coding-copilot.new.accessToken",
    "Tencent-Cloud.coding-copilot.accessToken",
    "planning-genie.new.accessTokencn",
    "planning-genie.new.accessToken",
];

const SECRET_STORAGE_FILE: &str = "secret-storage.xml";

/// IDEA 环境探测结果（用于前端诊断与提示）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebuddyIdeaEnv {
    /// 命中的 IDEA 配置目录（`<config>/options/secret-storage.xml` 的 `<config>`）
    pub config_dir: String,
    /// secret-storage.xml 路径
    pub secret_storage_path: String,
    /// secret-storage.xml 是否存在
    pub secret_storage_exists: bool,
    /// 是否找到了 CodeBuddy 插件的登录态条目
    pub secret_found: bool,
    /// 实际命中的条目 key
    pub secret_item_key: Option<String>,
}

// ─────────────────────────── 路径定位 ───────────────────────────

#[cfg(target_os = "windows")]
fn jetbrains_root() -> Result<PathBuf, String> {
    let appdata =
        std::env::var("APPDATA").map_err(|_| "无法获取 APPDATA 环境变量".to_string())?;
    Ok(PathBuf::from(appdata).join("JetBrains"))
}

#[cfg(target_os = "macos")]
fn jetbrains_root() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
    Ok(home
        .join("Library")
        .join("Application Support")
        .join("JetBrains"))
}

#[cfg(target_os = "linux")]
fn jetbrains_root() -> Result<PathBuf, String> {
    let base = if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg_config_home.trim();
        if trimmed.is_empty() {
            dirs::home_dir()
                .ok_or("无法获取用户主目录")?
                .join(".config")
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        dirs::home_dir()
            .ok_or("无法获取用户主目录")?
            .join(".config")
    };
    Ok(base.join("JetBrains"))
}

/// 所有 IntelliJ 系 IDE 配置目录，按最近修改时间倒序（最近使用过的排前面）。
pub fn config_dir_candidates() -> Vec<PathBuf> {
    let Ok(root) = jetbrains_root() else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs.sort_by_key(|path| Reverse(last_modified(path)));
    dirs
}

fn last_modified(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// 给定 IDE 配置目录，返回 secret-storage.xml 路径。
pub fn secret_storage_path(config_dir: &Path) -> PathBuf {
    config_dir.join("options").join(SECRET_STORAGE_FILE)
}

/// 候选的 secret-storage.xml 列表（按最近修改倒序）。
pub fn secret_storage_candidates() -> Vec<PathBuf> {
    config_dir_candidates()
        .into_iter()
        .map(|dir| secret_storage_path(&dir))
        .collect()
}

/// 从 secret-storage.xml 路径反推 IDEA 配置目录。
fn config_dir_of(file: &Path) -> String {
    file.parent()
        .and_then(|options| options.parent())
        .map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or_default()
}

// ─────────────────────────── XML 读写 ───────────────────────────

fn xml_unescape(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }
        let mut entity = String::new();
        let mut closed = false;
        while let Some(&next) = chars.peek() {
            chars.next();
            if next == ';' {
                closed = true;
                break;
            }
            entity.push(next);
            if entity.len() > 12 {
                break;
            }
        }
        if !closed {
            out.push('&');
            out.push_str(&entity);
            continue;
        }
        match entity.as_str() {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            other => {
                let decoded = other
                    .strip_prefix("#x")
                    .or_else(|| other.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| other.strip_prefix('#').and_then(|dec| dec.parse::<u32>().ok()))
                    .and_then(char::from_u32);
                match decoded {
                    Some(value) => out.push(value),
                    None => {
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                }
            }
        }
    }
    out
}

fn xml_escape_attr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            _ => out.push(ch),
        }
    }
    out
}

/// 解析一个 `<Entry ...>` 片段里的属性，返回 `(名称, 原始转义值, 值起始偏移, 值结束偏移)`。
fn parse_attrs(elem: &str) -> Vec<(&str, &str, usize, usize)> {
    let bytes = elem.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'>' {
            break;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'/'
            && bytes[i] != b'>'
        {
            i += 1;
        }
        let name = &elem[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let value_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                let value_end = i;
                if i < bytes.len() {
                    i += 1;
                }
                out.push((name, &elem[value_start..value_end], value_start, value_end));
            }
        }
    }
    out
}

/// 计算从 `start` 起一个 `<Entry>` 元素的结束偏移（相对整段文本）。
fn entry_end(text: &str, start: usize) -> usize {
    let rest = &text[start..];
    match (rest.find("/>"), rest.find("</Entry>")) {
        (Some(a), Some(b)) => start + a.min(b),
        (Some(a), None) => start + a,
        (None, Some(b)) => start + b,
        (None, None) => start + rest.find('>').unwrap_or(rest.len().saturating_sub(1)),
    }
}

/// 收集文件文本里所有 `<Entry>` 的 `(key, value)`（均已反转义）。
fn collect_entries(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("<Entry") {
        let start = from + rel;
        let end = entry_end(text, start);
        if end <= start {
            break;
        }
        let elem = &text[start..end];
        let attrs = parse_attrs(elem);
        let mut key = None;
        let mut value = None;
        for (name, raw, _, _) in attrs {
            if name == "key" {
                key = Some(xml_unescape(raw));
            } else if name == "value" {
                value = Some(xml_unescape(raw));
            }
        }
        if let (Some(k), Some(v)) = (key, value) {
            out.push((k, v));
        }
        from = end + 1;
    }
    out
}

/// 在文本中按优先级找出 CodeBuddy 登录态条目，返回 `(key, value_json)`。
fn find_secret_in_text(text: &str) -> Option<(String, String)> {
    let entries = collect_entries(text);
    for preferred in IDEA_SECRET_KEY_PREFERENCES {
        for (key, value) in &entries {
            if key.contains(preferred) && !value.is_empty() {
                return Some((key.clone(), value.clone()));
            }
        }
    }
    None
}

/// 读取单个文件里的 CodeBuddy 登录态，返回 `(key, value_json)`。
fn find_secret_in_file(file: &Path) -> Option<(String, String)> {
    let text = std::fs::read_to_string(file).ok()?;
    find_secret_in_text(&text)
}

/// 命中登录态的文件、条目 key 与会话 JSON。
fn find_session_file() -> Result<Option<(PathBuf, String, String)>, String> {
    for file in secret_storage_candidates() {
        if !file.exists() {
            continue;
        }
        if let Some((key, value)) = find_secret_in_file(&file) {
            return Ok(Some((file, key, value)));
        }
    }
    Ok(None)
}

/// 在文本里把指定 key 的 `<Entry>` 的 value 替换为 `new_value_escaped`。
fn replace_entry_value(text: &str, target_key: &str, new_value_escaped: &str) -> Option<String> {
    let mut from = 0usize;
    while let Some(rel) = text[from..].find("<Entry") {
        let start = from + rel;
        let end = entry_end(text, start);
        if end <= start {
            break;
        }
        let elem = &text[start..end];
        let attrs = parse_attrs(elem);
        let mut key = None;
        let mut value_span: Option<(usize, usize)> = None;
        for (name, raw, value_start, value_end) in attrs {
            if name == "key" {
                key = Some(xml_unescape(raw));
            } else if name == "value" {
                value_span = Some((start + value_start, start + value_end));
            }
        }
        if key.as_deref() == Some(target_key) {
            let (value_start, value_end) = value_span?;
            let mut out = String::with_capacity(text.len() + new_value_escaped.len());
            out.push_str(&text[..value_start]);
            out.push_str(new_value_escaped);
            out.push_str(&text[value_end..]);
            return Some(out);
        }
        from = end + 1;
    }
    None
}

// ─────────────────────────── 环境 / 登录态 ───────────────────────────

pub fn env_status() -> Result<CodebuddyIdeaEnv, String> {
    for file in secret_storage_candidates() {
        if !file.exists() {
            continue;
        }
        if let Some((key, _)) = find_secret_in_file(&file) {
            return Ok(CodebuddyIdeaEnv {
                config_dir: config_dir_of(&file),
                secret_storage_path: file.to_string_lossy().to_string(),
                secret_storage_exists: true,
                secret_found: true,
                secret_item_key: Some(key),
            });
        }
    }

    let candidates = secret_storage_candidates();
    let existing = candidates.iter().find(|path| path.exists()).cloned();
    let path = existing
        .clone()
        .or_else(|| candidates.into_iter().next())
        .ok_or_else(|| "未找到 IntelliJ 系 IDE 的配置目录（JetBrains）".to_string())?;

    Ok(CodebuddyIdeaEnv {
        config_dir: config_dir_of(&path),
        secret_storage_path: path.to_string_lossy().to_string(),
        secret_storage_exists: existing.is_some(),
        secret_found: false,
        secret_item_key: None,
    })
}

/// 读取 IDEA 里当前 CodeBuddy 插件的登录态明文（会话 JSON）。
pub fn read_current_session() -> Result<Option<String>, String> {
    Ok(find_session_file()?.map(|(_, _, value)| value))
}

/// 规范 token：剥离 `<uid>+` 前缀，并截断 `eyJ` 之前的前缀，只保留 JWT 本体。
fn normalize_idea_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = match trimmed.split_once('+') {
        Some((_, suffix)) => suffix.trim(),
        None => trimmed,
    };
    let candidate = match candidate.find("eyJ") {
        Some(index) if index > 0 => &candidate[index..],
        _ => candidate,
    };
    let candidate = candidate.trim();
    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

/// 从 IDEA 登录态里解析出 access token。
fn extract_access_token_from_session(secret: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(secret).ok();
    let raw = parsed
        .as_ref()
        .and_then(|value| {
            value
                .get("auth")
                .and_then(|auth| auth.get("accessToken"))
                .and_then(|token| token.as_str())
                .or_else(|| value.get("token").and_then(|token| token.as_str()))
                .or_else(|| value.get("accessToken").and_then(|token| token.as_str()))
        })
        .map(|token| token.to_string())
        .or_else(|| {
            let raw = secret.trim();
            if raw.is_empty() {
                None
            } else {
                Some(raw.to_string())
            }
        })?;
    normalize_idea_token(&raw)
}

/// 从 IDEA 登录态构造导入用的 payload（未落库）。
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
        .and_then(|value| {
            value
                .get("accessToken")
                .and_then(|token| token.as_str())
                .or_else(|| value.get("token").and_then(|token| token.as_str()))
                .or_else(|| {
                    value
                        .get("auth")
                        .and_then(|auth| auth.get("accessToken"))
                        .and_then(|token| token.as_str())
                })
        })
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .or_else(|| {
            let raw = secret.trim();
            if raw.is_empty() {
                None
            } else {
                Some(raw.to_string())
            }
        });

    let Some(raw_token) = token_candidate else {
        return Err("IDEA CodeBuddy 登录信息解析失败：未找到 access token".to_string());
    };

    let Some((uid_from_token, normalized_token)) =
        crate::modules::codebuddy_account::extract_local_codebuddy_token_parts(&raw_token)
    else {
        return Err("IDEA CodeBuddy 登录信息解析失败：access token 无效".to_string());
    };
    let Some(access_token) =
        crate::modules::codebuddy_account::normalize_local_codebuddy_token(&normalized_token)
    else {
        return Err("IDEA CodeBuddy 登录信息解析失败：access token 为空".to_string());
    };

    Ok(Some(
        crate::modules::codebuddy_account::build_local_import_payload(
            access_token,
            parsed_json,
            uid_from_token,
        ),
    ))
}

/// 生成写入 IDEA 的会话 JSON。
///
/// 优先在「IDEA 里已有的会话」基础上改写，只替换与凭据相关的字段，
/// 保留插件自己写入的其它字段；没有既有会话时才回落到桌面 IDE 同款的完整结构。
fn build_idea_session_json(account: &CodebuddyAccount, existing: Option<&str>) -> String {
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

/// 把指定账号写入 IDEA 里 CodeBuddy 插件的登录态。
///
/// 返回被切换账号的邮箱，便于前端直接展示提示。
pub fn inject_account_to_idea(account_id: &str) -> Result<String, String> {
    let account = crate::modules::codebuddy_cn_account::load_account(account_id)
        .ok_or_else(|| format!("CodeBuddy 账号不存在: {}", account_id))?;

    // 必须已存在插件写入过的登录态条目：只有这样才能拿到 IDEA 真实使用的条目 key，
    // 并据此在 XML 中原样回写。
    let Some((file, key, existing_value)) = find_session_file()? else {
        return Err(
            "未在 IntelliJ 系 IDE 中找到 CodeBuddy 插件的登录态（options/secret-storage.xml）。\n\n\
             请先在 IDEA 中安装并登录一次 CodeBuddy 插件，再使用切换功能。"
                .to_string(),
        );
    };

    let existing_session = if existing_value.trim().is_empty() {
        None
    } else {
        Some(existing_value.as_str())
    };
    let session_json = build_idea_session_json(&account, existing_session);
    let escaped = xml_escape_attr(&session_json);

    let text = std::fs::read_to_string(&file)
        .map_err(|err| format!("读取 IDEA 登录态文件失败: {} ({})", file.display(), err))?;
    let updated = replace_entry_value(&text, &key, &escaped).ok_or_else(|| {
        format!(
            "未能在 {} 中定位待写入的登录态条目: {}",
            file.display(),
            key
        )
    })?;
    crate::modules::atomic_write::write_string_atomic(&file, &updated)
        .map_err(|err| format!("写入 IDEA 登录态失败: {}", err))?;

    crate::modules::logger::log_info(&format!(
        "CodeBuddy IDEA 账号注入完成: email={}, file={}, key={}",
        account.email,
        file.to_string_lossy(),
        key
    ));

    Ok(account.email)
}

/// 在给定账号池中匹配出 IDEA 当前登录的账号 ID。
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<component name="SecretStorage">
  <Scores>
    <Entry key="Tencent-Cloud.coding-copilot.new.accessToken" value="{&quot;token&quot;:&quot;uid123+eyJhbGciOi.payload.sig&quot;,&quot;auth&quot;:{&quot;accessToken&quot;:&quot;eyJhbGciOi.payload.sig&quot;,&quot;expiresAt&quot;:123456}}" />
    <Entry key="other.key" value="x" />
  </Scores>
</component>"#;

    #[test]
    fn finds_and_unescapes_secret_entry() {
        let (key, value) = find_secret_in_text(SAMPLE).expect("find secret");
        assert_eq!(key, "Tencent-Cloud.coding-copilot.new.accessToken");
        assert!(value.contains("\"token\""));
        assert!(value.contains("eyJhbGciOi.payload.sig"));
    }

    #[test]
    fn extracts_normalized_token() {
        let (_, value) = find_secret_in_text(SAMPLE).expect("find secret");
        let token = extract_access_token_from_session(&value).expect("extract token");
        assert_eq!(token, "eyJhbGciOi.payload.sig");
    }

    #[test]
    fn replaces_entry_value_in_place() {
        let updated = replace_entry_value(
            SAMPLE,
            "Tencent-Cloud.coding-copilot.new.accessToken",
            "{&quot;token&quot;:&quot;new&quot;}",
        )
        .expect("replace");
        assert!(updated.contains("value=\"{&quot;token&quot;:&quot;new&quot;}\""));
        assert!(updated.contains(r#"value="x""#));
        // 其它条目不受影响
        let (_, value) = find_secret_in_text(&updated).expect("find secret again");
        assert!(value.contains("\"new\""));
    }

    #[test]
    fn normalize_handles_uid_prefix_and_plain_jwt() {
        assert_eq!(
            normalize_idea_token("uid123+eyJabc").as_deref(),
            Some("eyJabc")
        );
        assert_eq!(normalize_idea_token("eyJabc").as_deref(), Some("eyJabc"));
        assert_eq!(
            normalize_idea_token("prefixeyJabc").as_deref(),
            Some("eyJabc")
        );
    }
}
