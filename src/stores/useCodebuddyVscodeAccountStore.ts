import {
  CodebuddyAccount,
  getCodebuddyAccountDisplayEmail,
  getCodebuddyPlanBadge,
  getCodebuddyUsage,
} from '../types/codebuddy';
import * as codebuddyVscodeService from '../services/codebuddyVscodeService';
import { createProviderAccountStore } from './createProviderAccountStore';

const CODEBUDDY_VSCODE_ACCOUNTS_CACHE_KEY = 'agtools.codebuddyvscode.accounts.cache';
const CODEBUDDY_VSCODE_CURRENT_ACCOUNT_ID_KEY = 'agtools.codebuddyvscode.current_account_id';

/**
 * CodeBuddy VSCode 插件版账号 store。
 *
 * 账号池与 `codebuddy`（桌面 IDE）共用（后端同一份存储），
 * 区别在于「当前账号」以 VS Code 里实际登录的账号为准。
 */
export const useCodebuddyVscodeAccountStore = createProviderAccountStore<CodebuddyAccount>(
  CODEBUDDY_VSCODE_ACCOUNTS_CACHE_KEY,
  {
    listAccounts: codebuddyVscodeService.listCodebuddyVscodeAccounts,
    deleteAccount: codebuddyVscodeService.deleteCodebuddyVscodeAccount,
    deleteAccounts: codebuddyVscodeService.deleteCodebuddyVscodeAccounts,
    injectAccount: codebuddyVscodeService.injectCodebuddyVscodeAccount,
    refreshToken: codebuddyVscodeService.refreshCodebuddyVscodeToken,
    refreshAllTokens: codebuddyVscodeService.refreshAllCodebuddyVscodeTokens,
    importFromJson: codebuddyVscodeService.importCodebuddyVscodeFromJson,
    exportAccounts: codebuddyVscodeService.exportCodebuddyVscodeAccounts,
    updateAccountTags: codebuddyVscodeService.updateCodebuddyVscodeAccountTags,
  },
  {
    getDisplayEmail: getCodebuddyAccountDisplayEmail,
    getPlanBadge: getCodebuddyPlanBadge,
    getUsage: getCodebuddyUsage,
  },
  {
    platformId: 'codebuddy_vscode',
    currentAccountIdKey: CODEBUDDY_VSCODE_CURRENT_ACCOUNT_ID_KEY,
    resolveCurrentAccountId: () => codebuddyVscodeService.getCodebuddyVscodeCurrentAccountId(),
  },
);
