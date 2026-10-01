import { invoke } from '@tauri-apps/api/core';
import type { CodebuddyAccount } from '../types/codebuddy';

export interface CodebuddyVscodeOAuthLoginStartResponse {
  loginId: string;
  verificationUri: string;
  verificationUriComplete?: string | null;
  expiresIn: number;
  intervalSeconds: number;
}

export interface CodebuddyVscodeEnv {
  dataRoot: string;
  stateDbPath: string;
  stateDbExists: boolean;
  secretFound: boolean;
  secretItemKey?: string | null;
}

export async function listCodebuddyVscodeAccounts(): Promise<CodebuddyAccount[]> {
  return await invoke('list_codebuddy_vscode_accounts');
}

export async function deleteCodebuddyVscodeAccount(accountId: string): Promise<void> {
  return await invoke('delete_codebuddy_vscode_account', { accountId });
}

export async function deleteCodebuddyVscodeAccounts(accountIds: string[]): Promise<void> {
  return await invoke('delete_codebuddy_vscode_accounts', { accountIds });
}

export async function importCodebuddyVscodeFromJson(jsonContent: string): Promise<CodebuddyAccount[]> {
  return await invoke('import_codebuddy_vscode_from_json', { jsonContent });
}

export async function importCodebuddyVscodeFromLocal(): Promise<CodebuddyAccount[]> {
  return await invoke('import_codebuddy_vscode_from_local');
}

export async function exportCodebuddyVscodeAccounts(accountIds: string[]): Promise<string> {
  return await invoke('export_codebuddy_vscode_accounts', { accountIds });
}

export async function refreshCodebuddyVscodeToken(accountId: string): Promise<CodebuddyAccount> {
  return await invoke('refresh_codebuddy_vscode_token', { accountId });
}

export async function refreshAllCodebuddyVscodeTokens(): Promise<number> {
  return await invoke('refresh_all_codebuddy_vscode_tokens');
}

export async function startCodebuddyVscodeOAuthLogin(): Promise<CodebuddyVscodeOAuthLoginStartResponse> {
  return await invoke('codebuddy_vscode_oauth_login_start');
}

export async function completeCodebuddyVscodeOAuthLogin(loginId: string): Promise<CodebuddyAccount> {
  return await invoke('codebuddy_vscode_oauth_login_complete', { loginId });
}

export async function cancelCodebuddyVscodeOAuthLogin(loginId?: string): Promise<void> {
  return await invoke('codebuddy_vscode_oauth_login_cancel', { loginId: loginId ?? null });
}

export async function addCodebuddyVscodeAccountWithToken(accessToken: string): Promise<CodebuddyAccount> {
  return await invoke('add_codebuddy_vscode_account_with_token', { accessToken });
}

export async function updateCodebuddyVscodeAccountTags(
  accountId: string,
  tags: string[],
): Promise<CodebuddyAccount> {
  return await invoke('update_codebuddy_vscode_account_tags', { accountId, tags });
}

export async function getCodebuddyVscodeAccountsIndexPath(): Promise<string> {
  return await invoke('get_codebuddy_vscode_accounts_index_path');
}

/** 探测本机 VS Code 与 CodeBuddy 扩展登录态位置。 */
export async function getCodebuddyVscodeEnv(): Promise<CodebuddyVscodeEnv> {
  return await invoke('get_codebuddy_vscode_env');
}

/** 读取 VS Code 当前登录的 CodeBuddy 账号 ID。 */
export async function getCodebuddyVscodeCurrentAccountId(): Promise<string | null> {
  return await invoke('get_codebuddy_vscode_current_account_id');
}

/** 把账号池中的账号切换到 VS Code 的 CodeBuddy 扩展。 */
export async function injectCodebuddyVscodeAccount(accountId: string): Promise<string> {
  return await invoke('inject_codebuddy_vscode_account', { accountId });
}
