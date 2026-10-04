import {
  CodebuddyAccount,
  getCodebuddyAccountDisplayEmail,
  getCodebuddyPlanBadge,
  getCodebuddyUsage,
} from '../types/codebuddy';
import * as codebuddyIdeaService from '../services/codebuddyIdeaService';
import { createProviderAccountStore } from './createProviderAccountStore';

const CODEBUDDY_IDEA_ACCOUNTS_CACHE_KEY = 'agtools.codebuddyidea.accounts.cache';
const CODEBUDDY_IDEA_CURRENT_ACCOUNT_ID_KEY = 'agtools.codebuddyidea.current_account_id';

/**
 * CodeBuddy IDEA（IntelliJ IDEA 插件版）账号 store。
 *
 * 账号池与 `codebuddy_cn`（国内版桌面 IDE）共用（后端同一份存储），
 * 区别在于「当前账号」以 IDEA 里实际登录的账号为准。
 */
export const useCodebuddyIdeaAccountStore = createProviderAccountStore<CodebuddyAccount>(
  CODEBUDDY_IDEA_ACCOUNTS_CACHE_KEY,
  {
    listAccounts: codebuddyIdeaService.listCodebuddyIdeaAccounts,
    deleteAccount: codebuddyIdeaService.deleteCodebuddyIdeaAccount,
    deleteAccounts: codebuddyIdeaService.deleteCodebuddyIdeaAccounts,
    injectAccount: codebuddyIdeaService.injectCodebuddyIdeaAccount,
    refreshToken: codebuddyIdeaService.refreshCodebuddyIdeaToken,
    refreshAllTokens: codebuddyIdeaService.refreshAllCodebuddyIdeaTokens,
    importFromJson: codebuddyIdeaService.importCodebuddyIdeaFromJson,
    exportAccounts: codebuddyIdeaService.exportCodebuddyIdeaAccounts,
    updateAccountTags: codebuddyIdeaService.updateCodebuddyIdeaAccountTags,
  },
  {
    getDisplayEmail: getCodebuddyAccountDisplayEmail,
    getPlanBadge: getCodebuddyPlanBadge,
    getUsage: getCodebuddyUsage,
  },
  {
    platformId: 'codebuddy_idea',
    currentAccountIdKey: CODEBUDDY_IDEA_CURRENT_ACCOUNT_ID_KEY,
    resolveCurrentAccountId: () => codebuddyIdeaService.getCodebuddyIdeaCurrentAccountId(),
  },
);
