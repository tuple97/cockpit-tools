import { invoke } from '@tauri-apps/api/core';
import type { CodebuddyAccount } from '../types/codebuddy';

export interface CodebuddyIdeaOAuthLoginStartResponse {
  loginId: string;
  verificationUri: string;
  verificationUriComplete?: string | null;
  expiresIn: number;
  intervalSeconds: number;
}

export interface CodebuddyIdeaEnv {
  configDir: string;
  secretStoragePath: string;
  secretStorageExists: boolean;
  secretFound: boolean;
  secretItemKey?: string | null;
}

export async function listCodebuddyIdeaAccounts(): Promise<CodebuddyAccount[]> {
  return await invoke('list_codebuddy_idea_accounts');
}

export async function deleteCodebuddyIdeaAccount(accountId: string): Promise<void> {
  return await invoke('delete_codebuddy_idea_account', { accountId });
}

export async function deleteCodebuddyIdeaAccounts(accountIds: string[]): Promise<void> {
  return await invoke('delete_codebuddy_idea_accounts', { accountIds });
}

export async function importCodebuddyIdeaFromJson(jsonContent: string): Promise<CodebuddyAccount[]> {
  return await invoke('import_codebuddy_idea_from_json', { jsonContent });
}

export async function importCodebuddyIdeaFromLocal(): Promise<CodebuddyAccount[]> {
  return await invoke('import_codebuddy_idea_from_local');
}

export async function exportCodebuddyIdeaAccounts(accountIds: string[]): Promise<string> {
  return await invoke('export_codebuddy_idea_accounts', { accountIds });
}

export async function refreshCodebuddyIdeaToken(accountId: string): Promise<CodebuddyAccount> {
  return await invoke('refresh_codebuddy_idea_token', { accountId });
}

export async function refreshAllCodebuddyIdeaTokens(): Promise<number> {
  return await invoke('refresh_all_codebuddy_idea_tokens');
}

export async function startCodebuddyIdeaOAuthLogin(): Promise<CodebuddyIdeaOAuthLoginStartResponse> {
  return await invoke('codebuddy_idea_oauth_login_start');
}

export async function completeCodebuddyIdeaOAuthLogin(loginId: string): Promise<CodebuddyAccount> {
  return await invoke('codebuddy_idea_oauth_login_complete', { loginId });
}

export async function cancelCodebuddyIdeaOAuthLogin(loginId?: string): Promise<void> {
  return await invoke('codebuddy_idea_oauth_login_cancel', { loginId: loginId ?? null });
}

export async function addCodebuddyIdeaAccountWithToken(accessToken: string): Promise<CodebuddyAccount> {
  return await invoke('add_codebuddy_idea_account_with_token', { accessToken });
}

export async function updateCodebuddyIdeaAccountTags(
  accountId: string,
  tags: string[],
): Promise<CodebuddyAccount> {
  return await invoke('update_codebuddy_idea_account_tags', { accountId, tags });
}

export async function getCodebuddyIdeaAccountsIndexPath(): Promise<string> {
  return await invoke('get_codebuddy_idea_accounts_index_path');
}

/** 探测本机 IntelliJ 系 IDE 与 CodeBuddy 插件登录态位置。 */
export async function getCodebuddyIdeaEnv(): Promise<CodebuddyIdeaEnv> {
  return await invoke('get_codebuddy_idea_env');
}

/** 读取 IDEA 中当前登录的 CodeBuddy 账号 ID。 */
export async function getCodebuddyIdeaCurrentAccountId(): Promise<string | null> {
  return await invoke('get_codebuddy_idea_current_account_id');
}

/** 把账号池中的账号切换到 IDEA 的 CodeBuddy 插件。 */
export async function injectCodebuddyIdeaAccount(accountId: string): Promise<string> {
  return await invoke('inject_codebuddy_idea_account', { accountId });
}
