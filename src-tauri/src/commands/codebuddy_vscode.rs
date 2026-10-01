//! CodeBuddy VSCode 插件版平台命令。
//!
//! 本平台走国内版（codebuddy.cn）站点：账号池与 `codebuddy_cn` 共用，因此列表 /
//! 增删 / 刷新 / OAuth 等命令直接委托给 `codebuddy_cn_account` 与 `codebuddy_cn_oauth`；
//! 仅「读写 VS Code 登录态」与「当前账号」使用 `codebuddy_vscode` 模块的定制实现。

use tauri::AppHandle;

use crate::models::codebuddy::{CodebuddyAccount, CodebuddyOAuthStartResponse};
use crate::modules::{codebuddy_cn_account, codebuddy_cn_oauth, codebuddy_vscode, logger};

const CODEBUDDY_VSCODE_PLATFORM: &str = "codebuddy_vscode";

async fn refresh_account_after_login(account: CodebuddyAccount) -> CodebuddyAccount {
    let account_id = account.id.clone();
    match codebuddy_cn_account::refresh_account_token(&account_id).await {
        Ok(refreshed) => refreshed,
        Err(e) => {
            logger::log_warn(&format!(
                "[CodeBuddyVscode OAuth] 登录后刷新失败，保留原账号信息: account_id={}, error={}",
                account_id, e
            ));
            account
        }
    }
}

#[tauri::command]
pub fn list_codebuddy_vscode_accounts() -> Result<Vec<CodebuddyAccount>, String> {
    codebuddy_cn_account::list_accounts_checked()
}

#[tauri::command]
pub fn delete_codebuddy_vscode_account(account_id: String) -> Result<(), String> {
    codebuddy_cn_account::remove_account(&account_id)
}

#[tauri::command]
pub fn delete_codebuddy_vscode_accounts(account_ids: Vec<String>) -> Result<(), String> {
    codebuddy_cn_account::remove_accounts(&account_ids)
}

#[tauri::command]
pub fn import_codebuddy_vscode_from_json(
    json_content: String,
) -> Result<Vec<CodebuddyAccount>, String> {
    codebuddy_cn_account::import_from_json(&json_content)
}

#[tauri::command]
pub async fn import_codebuddy_vscode_from_local(
    app: AppHandle,
) -> Result<Vec<CodebuddyAccount>, String> {
    let mut local_payload = match codebuddy_vscode::import_payload_from_local()? {
        Some(payload) => payload,
        None => return Err("未在 VS Code 的 CodeBuddy 扩展中找到登录信息".to_string()),
    };

    match codebuddy_cn_oauth::build_payload_from_token(&local_payload.access_token).await {
        Ok(mut payload) => {
            if payload.uid.is_none() {
                payload.uid = local_payload.uid.clone();
            }
            if payload.nickname.is_none() {
                payload.nickname = local_payload.nickname.clone();
            }
            if payload.refresh_token.is_none() {
                payload.refresh_token = local_payload.refresh_token.clone();
            }
            if payload.domain.is_none() {
                payload.domain = local_payload.domain.clone();
            }
            if payload.token_type.is_none() {
                payload.token_type = local_payload.token_type.clone();
            }
            if payload.expires_at.is_none() {
                payload.expires_at = local_payload.expires_at;
            }
            if payload.auth_raw.is_none() {
                payload.auth_raw = local_payload.auth_raw.clone();
            }
            if payload.profile_raw.is_none() {
                payload.profile_raw = local_payload.profile_raw.clone();
            }
            if payload.email.trim().is_empty() || payload.email == "unknown" {
                payload.email = local_payload.email.clone();
            }
            local_payload = payload;
        }
        Err(err) => {
            logger::log_warn(&format!(
                "[CodeBuddyVscode Import Local] 拉取账号资料失败，将保留本地导入结果: {}",
                err
            ));
        }
    }

    let mut account = codebuddy_cn_account::upsert_account(local_payload.clone())?;
    account = refresh_account_after_login(account).await;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(vec![account])
}

#[tauri::command]
pub fn export_codebuddy_vscode_accounts(account_ids: Vec<String>) -> Result<String, String> {
    codebuddy_cn_account::export_accounts(&account_ids)
}

#[tauri::command]
pub async fn refresh_codebuddy_vscode_token(
    app: AppHandle,
    account_id: String,
) -> Result<CodebuddyAccount, String> {
    let account = codebuddy_cn_account::refresh_account_token(&account_id).await?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(account)
}

#[tauri::command]
pub async fn refresh_all_codebuddy_vscode_tokens(app: AppHandle) -> Result<i32, String> {
    let results = codebuddy_cn_account::refresh_all_tokens().await?;
    let success_count = results.iter().filter(|(_, item)| item.is_ok()).count();
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(success_count as i32)
}

#[tauri::command]
pub async fn codebuddy_vscode_oauth_login_start() -> Result<CodebuddyOAuthStartResponse, String> {
    logger::log_info("CodeBuddy VSCode OAuth start 命令触发");
    codebuddy_cn_oauth::start_login().await
}

#[tauri::command]
pub async fn codebuddy_vscode_oauth_login_complete(
    app: AppHandle,
    login_id: String,
) -> Result<CodebuddyAccount, String> {
    logger::log_info(&format!(
        "CodeBuddy VSCode OAuth complete 命令触发: login_id={}",
        login_id
    ));

    let result: Result<CodebuddyAccount, String> = async {
        let payload = codebuddy_cn_oauth::complete_login(&login_id).await?;
        let mut account = codebuddy_cn_account::upsert_account(payload)?;
        account = refresh_account_after_login(account).await;
        Ok(account)
    }
    .await;

    if let Err(err) = codebuddy_cn_oauth::clear_pending_oauth_login(&login_id) {
        logger::log_warn(&format!(
            "[CodeBuddyVscode OAuth] 清理待处理登录状态失败: login_id={}, error={}",
            login_id, err
        ));
    }

    let account = result?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(account)
}

#[tauri::command]
pub fn codebuddy_vscode_oauth_login_cancel(login_id: Option<String>) -> Result<(), String> {
    codebuddy_cn_oauth::cancel_login(login_id.as_deref())
}

#[tauri::command]
pub async fn add_codebuddy_vscode_account_with_token(
    app: AppHandle,
    access_token: String,
) -> Result<CodebuddyAccount, String> {
    let payload = codebuddy_cn_oauth::build_payload_from_token(&access_token).await?;
    let account = codebuddy_cn_account::upsert_account(payload)?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(account)
}

#[tauri::command]
pub async fn update_codebuddy_vscode_account_tags(
    account_id: String,
    tags: Vec<String>,
) -> Result<CodebuddyAccount, String> {
    codebuddy_cn_account::update_account_tags(&account_id, tags)
}

#[tauri::command]
pub fn get_codebuddy_vscode_accounts_index_path() -> Result<String, String> {
    codebuddy_cn_account::accounts_index_path_string()
}

/// 探测本机 VS Code 环境与 CodeBuddy 扩展登录态位置。
#[tauri::command]
pub fn get_codebuddy_vscode_env() -> Result<codebuddy_vscode::CodebuddyVscodeEnv, String> {
    codebuddy_vscode::env_status()
}

/// 读取 VS Code 当前登录的 CodeBuddy 账号 ID。
///
/// VS Code 未安装 / 未登录 / 系统凭据不可用时返回 `None`（而不是报错），
/// 以免前端因偶发的钥匙串拒绝而清掉本地缓存的当前账号。
#[tauri::command]
pub fn get_codebuddy_vscode_current_account_id() -> Result<Option<String>, String> {
    let accounts = codebuddy_cn_account::list_accounts();
    match codebuddy_vscode::resolve_current_account_id(&accounts) {
        Ok(account_id) => Ok(account_id),
        Err(err) => {
            logger::log_warn(&format!(
                "[CodeBuddyVscode] 读取 VS Code 当前账号失败: {}",
                err
            ));
            Ok(None)
        }
    }
}

/// 把账号池中的某个账号切换到 VS Code 的 CodeBuddy 扩展。
#[tauri::command]
pub async fn inject_codebuddy_vscode_account(
    app: AppHandle,
    account_id: String,
) -> Result<String, String> {
    let email = codebuddy_vscode::inject_account_to_vscode(&account_id)?;
    crate::modules::provider_current_state::set_current_account_id(
        CODEBUDDY_VSCODE_PLATFORM,
        Some(account_id.as_str()),
    )?;
    let _ = crate::modules::tray::update_tray_menu(&app);
    Ok(format!("切换完成: {}", email))
}
