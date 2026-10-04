import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  PlatformOverviewTabsHeader,
  PlatformOverviewTab,
} from '../components/platform/PlatformOverviewTabsHeader';
import { useCodebuddyIdeaAccountStore } from '../stores/useCodebuddyIdeaAccountStore';
import * as codebuddyIdeaService from '../services/codebuddyIdeaService';
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

const CB_IDEA_FLOW_NOTICE_COLLAPSED_KEY = 'agtools.codebuddyidea.flow_notice_collapsed';
const CB_IDEA_CURRENT_ACCOUNT_ID_KEY = 'agtools.codebuddyidea.current_account_id';

const codebuddyIdeaPlatformConfig: CodebuddySuiteAccountsPlatformConfig<CodebuddyAccount> = {
  pageClassName: 'codebuddy-accounts-page',
  searchPlaceholderKey: 'codebuddyIdea.search',
  searchPlaceholderDefault: '搜索 CodeBuddy 账号...',
  flowNotice: {
    titleKey: 'codebuddyIdea.flowNotice.title',
    titleDefault: 'CodeBuddy IDEA 插件版说明（点击展开/收起）',
    descKey: 'codebuddyIdea.flowNotice.desc',
    descDefault:
      '此平台用于切换 IntelliJ IDEA 等 JetBrains IDE 里 CodeBuddy 插件的登录账号：读取/回写 IDE 配置目录下的 options/secret-storage.xml 登录态，数据仅在本地处理。走国内版站点，账号与 CodeBuddy CN（国内版）共用同一份账号池。',
    permissionKey: 'codebuddyIdea.flowNotice.permission',
    permissionDefault:
      '权限范围：读取并回写 IntelliJ 系 IDE 配置目录下的 options/secret-storage.xml（纯 XML，未加密）。',
    networkKey: 'codebuddyIdea.flowNotice.network',
    networkDefault:
      '网络范围：OAuth 授权登录与 Token 刷新需联网请求 codebuddy.cn 与计费接口。不上传本地密钥或凭证。',
  },
  noAccountsKey: 'codebuddyIdea.noAccounts',
  noAccountsDefault: '暂无 CodeBuddy 账号',
  addAccountTitleKey: 'codebuddyIdea.addAccount',
  addAccountTitleDefault: '添加 CodeBuddy 账号',
  oauthDescKey: 'codebuddyIdea.oauthDesc',
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
  tokenDescKey: 'codebuddyIdea.tokenDesc',
  tokenDescDefault: '粘贴 CodeBuddy 的 access token：',
  importLocalDescKey: 'codebuddyIdea.import.localDesc',
  importLocalDescDefault:
    '支持从本机 IntelliJ 系 IDE 的 CodeBuddy 插件或 JSON 文件导入账号数据。切换账号后需要重新加载 IDE 生效。',
  importLocalClientKey: 'codebuddyIdea.import.localClient',
  importLocalClientDefault: '从本机 IDEA 导入',
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

export function CodebuddyIdeaAccountsPage() {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<PlatformOverviewTab>('overview');
  const store = useCodebuddyIdeaAccountStore();
  const [env, setEnv] = useState<codebuddyIdeaService.CodebuddyIdeaEnv | null>(null);

  useEffect(() => {
    let cancelled = false;
    codebuddyIdeaService
      .getCodebuddyIdeaEnv()
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
    platformKey: 'CodeBuddyIdea',
    oauthLogPrefix: 'CodebuddyIdeaOAuth',
    flowNoticeCollapsedKey: CB_IDEA_FLOW_NOTICE_COLLAPSED_KEY,
    currentAccountIdKey: CB_IDEA_CURRENT_ACCOUNT_ID_KEY,
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
      startLogin: codebuddyIdeaService.startCodebuddyIdeaOAuthLogin,
      completeLogin: codebuddyIdeaService.completeCodebuddyIdeaOAuthLogin,
      cancelLogin: codebuddyIdeaService.cancelCodebuddyIdeaOAuthLogin,
    },
    dataService: {
      importFromJson: codebuddyIdeaService.importCodebuddyIdeaFromJson,
      importFromLocal: codebuddyIdeaService.importCodebuddyIdeaFromLocal,
      addWithToken: codebuddyIdeaService.addCodebuddyIdeaAccountWithToken,
      exportAccounts: codebuddyIdeaService.exportCodebuddyIdeaAccounts,
      injectToVSCode: codebuddyIdeaService.injectCodebuddyIdeaAccount,
    },
    getDisplayEmail: (account) => getCodebuddyAccountDisplayEmail(account),
  });

  const showEnvHint = env != null && (!env.secretStorageExists || !env.secretFound);

  return (
    <div className={`ghcp-accounts-page ${codebuddyIdeaPlatformConfig.pageClassName}`}>
      <PlatformOverviewTabsHeader
        platform="codebuddy_idea"
        active={activeTab}
        onTabChange={setActiveTab}
        tabs={['overview']}
      />
      {showEnvHint && (
        <div className="codebuddy-vscode-env-hint">
          {env?.secretFound
            ? t('codebuddyIdea.env.noSecretStorage', '未检测到 IDEA 登录态文件：')
            : t('codebuddyIdea.env.noSecret', '未检测到 IDEA 中 CodeBuddy 插件的登录态：')}
          <code>{env?.secretStoragePath}</code>
          <span>
            {t(
              'codebuddyIdea.env.hint',
              '请先在 IntelliJ IDEA 中安装并登录一次 CodeBuddy 插件，再使用切换功能。',
            )}
          </span>
        </div>
      )}
      <CodebuddySuiteAccountsSharedView
        accounts={store.accounts}
        loading={store.loading}
        page={page}
        platformConfig={codebuddyIdeaPlatformConfig}
        onRefreshAccounts={() => {
          store.fetchAccounts();
        }}
      />
    </div>
  );
}
