import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  PlatformOverviewTabsHeader,
  PlatformOverviewTab,
} from '../components/platform/PlatformOverviewTabsHeader';
import { useCodebuddyVscodeAccountStore } from '../stores/useCodebuddyVscodeAccountStore';
import * as codebuddyVscodeService from '../services/codebuddyVscodeService';
import {
  CodebuddyAccount,
  getCodebuddyAccountDisplayEmail,
  getCodebuddyOfficialQuotaModel,
  getCodebuddyPlanBadge,
  getCodebuddyUsage,
  getCodebuddyQuotaCategoryGroups,
} from '../types/codebuddy';
import { useProviderAccountsPage } from '../hooks/useProviderAccountsPage';
import {
  CodebuddySuiteAccountsSharedView,
  type CodebuddySuiteAccountsPlatformConfig,
} from '../components/codebuddy-suite/CodebuddySuiteAccountsSharedView';

const CB_VSCODE_FLOW_NOTICE_COLLAPSED_KEY = 'agtools.codebuddyvscode.flow_notice_collapsed';
const CB_VSCODE_CURRENT_ACCOUNT_ID_KEY = 'agtools.codebuddyvscode.current_account_id';

const codebuddyVscodePlatformConfig: CodebuddySuiteAccountsPlatformConfig<CodebuddyAccount> = {
  pageClassName: 'codebuddy-accounts-page',
  searchPlaceholderKey: 'codebuddyVscode.search',
  searchPlaceholderDefault: '搜索 CodeBuddy 账号...',
  flowNotice: {
    titleKey: 'codebuddyVscode.flowNotice.title',
    titleDefault: 'CodeBuddy VSCode 插件版说明（点击展开/收起）',
    descKey: 'codebuddyVscode.flowNotice.desc',
    descDefault:
      '此平台用于切换 VS Code 本体里 CodeBuddy 扩展的登录账号：读取/回写 VS Code 的 state.vscdb 登录态，数据仅在本地处理。账号与 CodeBuddy（桌面 IDE）共用同一份账号池。',
    permissionKey: 'codebuddyVscode.flowNotice.permission',
    permissionDefault:
      '权限范围：读取并回写 VS Code 认证数据库 (state.vscdb)，调用系统凭据能力（macOS Keychain / Windows DPAPI / Linux Secret Service）进行解密/加密。',
    networkKey: 'codebuddyVscode.flowNotice.network',
    networkDefault:
      '网络范围：OAuth 授权登录与 Token 刷新需联网请求 codebuddy.ai 与计费接口。不上传本地密钥或凭证。',
  },
  noAccountsKey: 'codebuddyVscode.noAccounts',
  noAccountsDefault: '暂无 CodeBuddy 账号',
  addAccountTitleKey: 'codebuddyVscode.addAccount',
  addAccountTitleDefault: '添加 CodeBuddy 账号',
  oauthDescKey: 'codebuddyVscode.oauthDesc',
  oauthDescDefault: '点击下方按钮将在浏览器中打开 CodeBuddy 授权页面。',
  oauthFeatureCardClassName: 'codebuddy-oauth-feature-card',
  oauthFeatureTitleKey: 'codebuddy.oauthFeature.oauth.title',
  oauthFeatureTitleDefault: '仅授权 IDE 登录信息',
  oauthFeatureItem1Key: 'codebuddy.oauthFeature.oauth.item1',
  oauthFeatureItem1Default: '在浏览器完成 OAuth 后即可添加账号并用于切换。',
  oauthFeatureItem2Key: 'codebuddy.oauthFeature.oauth.item2',
  oauthFeatureItem2Default: '授权完成后会自动刷新资源包配额数据。',
  oauthFeatureItem3Key: 'codebuddy.oauthFeature.oauth.item3',
  oauthFeatureItem3Default: '账号卡片将按资源包展示额度、进度和刷新/到期时间。',
  oauthUrlInputPlaceholderKey: 'codebuddy.oauthUrlInputPlaceholder',
  oauthUrlInputPlaceholderDefault: '可手动输入授权地址',
  oauthWaitingKey: 'codebuddy.oauthWaiting',
  oauthWaitingDefault: '等待授权完成...',
  tokenDescKey: 'codebuddyVscode.tokenDesc',
  tokenDescDefault: '粘贴 CodeBuddy 的 access token：',
  importLocalDescKey: 'codebuddyVscode.import.localDesc',
  importLocalDescDefault:
    '支持从本机 VS Code 的 CodeBuddy 扩展或 JSON 文件导入账号数据。切换账号后需要重新加载 VS Code 窗口生效。',
  importLocalClientKey: 'codebuddyVscode.import.localClient',
  importLocalClientDefault: '从本机 VS Code 导入',
  getDisplayEmail: (account) => getCodebuddyAccountDisplayEmail(account),
  getPlanBadge: (account) => getCodebuddyPlanBadge(account),
  getUsage: (account) => getCodebuddyUsage(account),
  getQuotaGroups: (account, t) => getCodebuddyQuotaCategoryGroups(account, t),
  hasQuotaData: (account) => {
    const model = getCodebuddyOfficialQuotaModel(account);
    return (
      model.resources.length > 0 ||
      model.extra.total > 0 ||
      model.extra.remain > 0 ||
      model.extra.used > 0
    );
  },
  usagePrefix: 'codebuddy',
  quotaPrefix: 'codebuddy',
  tableUsageClassName: 'codebuddy-table-usage',
};

export function CodebuddyVscodeAccountsPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<PlatformOverviewTab>('overview');
  const store = useCodebuddyVscodeAccountStore();
  const [env, setEnv] = useState<codebuddyVscodeService.CodebuddyVscodeEnv | null>(null);

  useEffect(() => {
    let cancelled = false;
    codebuddyVscodeService
      .getCodebuddyVscodeEnv()
      .then((value) => {
        if (!cancelled) setEnv(value);
      })
      .catch(() => {
        if (!cancelled) setEnv(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const page = useProviderAccountsPage<CodebuddyAccount>({
    platformKey: 'CodeBuddyVscode',
    oauthLogPrefix: 'CodebuddyVscodeOAuth',
    flowNoticeCollapsedKey: CB_VSCODE_FLOW_NOTICE_COLLAPSED_KEY,
    currentAccountIdKey: CB_VSCODE_CURRENT_ACCOUNT_ID_KEY,
    exportFilePrefix: 'codebuddy_accounts',
    oauthTabKeys: ['oauth'],
    store: {
      accounts: store.accounts,
      currentAccountId: store.currentAccountId,
      loading: store.loading,
      error: store.error,
      fetchAccounts: store.fetchAccounts,
      fetchCurrentAccountId: store.fetchCurrentAccountId,
      deleteAccounts: store.deleteAccounts,
      refreshToken: store.refreshToken,
      refreshAllTokens: store.refreshAllTokens,
      setCurrentAccountId: store.setCurrentAccountId,
      updateAccountTags: store.updateAccountTags,
    },
    oauthService: {
      startLogin: codebuddyVscodeService.startCodebuddyVscodeOAuthLogin,
      completeLogin: codebuddyVscodeService.completeCodebuddyVscodeOAuthLogin,
      cancelLogin: codebuddyVscodeService.cancelCodebuddyVscodeOAuthLogin,
    },
    dataService: {
      importFromJson: codebuddyVscodeService.importCodebuddyVscodeFromJson,
      importFromLocal: codebuddyVscodeService.importCodebuddyVscodeFromLocal,
      addWithToken: codebuddyVscodeService.addCodebuddyVscodeAccountWithToken,
      exportAccounts: codebuddyVscodeService.exportCodebuddyVscodeAccounts,
      injectToVSCode: codebuddyVscodeService.injectCodebuddyVscodeAccount,
    },
    getDisplayEmail: (account) => getCodebuddyAccountDisplayEmail(account),
  });

  const showEnvHint = env != null && (!env.stateDbExists || !env.secretFound);

  return (
    <div className={`ghcp-accounts-page ${codebuddyVscodePlatformConfig.pageClassName}`}>
      <PlatformOverviewTabsHeader
        platform="codebuddy_vscode"
        active={activeTab}
        onTabChange={setActiveTab}
        tabs={['overview']}
      />
      {showEnvHint && (
        <div className="codebuddy-vscode-env-hint">
          {env?.secretFound
            ? t('codebuddyVscode.env.noStateDb', '未检测到 VS Code 登录数据库：')
            : t('codebuddyVscode.env.noSecret', '未检测到 VS Code 中 CodeBuddy 扩展的登录态：')}
          <code>{env?.stateDbPath}</code>
          <span>
            {t(
              'codebuddyVscode.env.hint',
              '请先在 VS Code 中安装并登录一次 CodeBuddy 扩展，再使用切换功能。',
            )}
          </span>
        </div>
      )}
      <CodebuddySuiteAccountsSharedView
        accounts={store.accounts}
        loading={store.loading}
        page={page}
        platformConfig={codebuddyVscodePlatformConfig}
        onRefreshAccounts={() => {
          store.fetchAccounts();
        }}
      />
    </div>
  );
}
