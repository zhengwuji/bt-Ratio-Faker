<script>
  import Card from '$lib/components/ui/card.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import Input from '$lib/components/ui/input.svelte';
  import Checkbox from '$lib/components/ui/checkbox.svelte';
  import InlineHelp from '$lib/components/common/InlineHelp.svelte';
  import { cn } from '$lib/utils.js';
  import { extractTrackerHost } from '$lib/trackerUtils.js';
  import { instances as instancesStore } from '$lib/instanceStore.js';
  import { Settings, ArrowUpDown, Clock, Timer, Upload, Download, Lock, Fingerprint, Globe, CheckCircle2, Ban, ExternalLink, ChevronDown } from '@lucide/svelte';
  import ClientIcon from './ClientIcon.svelte';
  import ClientSelect from './ClientSelect.svelte';
  import VersionSelect from './VersionSelect.svelte';
  import RandomizationSettings from './RandomizationSettings.svelte';
  import ProgressiveRateSettings from './ProgressiveRateSettings.svelte';

  let {
    clients,
    clientVersions,
    selectedClient,
    selectedClientVersion,
    port,
    vpnPortSyncVisible = false,
    currentForwardedPort = null,
    vpnPortSyncEnabled = true,
    networkStatusConfigured = true,
    networkStatusError = null,
    vpnPortSync,
    uploadRate,
    downloadRate,
    completionPercent,
    initialUploaded,
    updateIntervalSeconds,
    scrapeInterval,
    randomizeRates,
    randomRangePercent,
    progressiveRatesEnabled,
    targetUploadRate,
    targetDownloadRate,
    progressiveDurationHours,
    peerIdPattern = '',
    keyPattern = '',
    proxyUrl = '',
    scheduleSlowEnabled = false,
    scheduleSlowStartUtc = 15,
    scheduleSlowEndUtc = 0,
    scheduleSlowScale = 0.3,
    mrProfiles = [],
    isRunning,
    onUpdate,
  } = $props();

  // Local state for form values (defaults match createDefaultInstance)
  let localSelectedClient = $state('qbittorrent');
  let localSelectedClientVersion = $state(null);
  let localPort = $state(6881);
  let localVpnPortSync = $state(false);
  let localUploadRate = $state(50);
  let localDownloadRate = $state(100);
  let localCompletionPercent = $state(0);
  let localInitialUploaded = $state(0);
  let localUpdateIntervalSeconds = $state(5);
  let localScrapeInterval = $state(60);
  let localRandomizeRates = $state(true);
  let localRandomRangePercent = $state(20);
  let localProgressiveRatesEnabled = $state(false);
  let localTargetUploadRate = $state(100);
  let localTargetDownloadRate = $state(200);
  let localProgressiveDurationHours = $state(1);
  let localPeerIdPattern = $state('');
  let localKeyPattern = $state('');
  let localProxyUrl = $state('');
  // 结构化代理输入(协议/地址/端口/用户名/密码)
  let proxyScheme = $state('socks5');
  let proxyHost = $state('');
  let proxyPort = $state('');
  let proxyUser = $state('');
  let proxyPass = $state('');
  let proxyTestResult = $state(null); // { ok, msg }
  let proxyTesting = $state(false);

  // Track if we're currently editing to prevent external updates from interfering
  let isEditing = $state(false);

  // Update local state when props change (only when not actively editing)
  $effect(() => {
    if (!isEditing) {
      localSelectedClient = selectedClient;
      localSelectedClientVersion = selectedClientVersion;
      localVpnPortSync = vpnPortSyncVisible ? (vpnPortSync ?? false) : false;
      localPort =
        vpnPortSyncVisible && localVpnPortSync && vpnPortSyncEnabled && currentForwardedPort
          ? currentForwardedPort
          : port;
      localUploadRate = uploadRate;
      localDownloadRate = downloadRate;
      localCompletionPercent = completionPercent;
      localInitialUploaded = initialUploaded;
      localUpdateIntervalSeconds = updateIntervalSeconds;
      localScrapeInterval = scrapeInterval;
      localRandomizeRates = randomizeRates;
      localRandomRangePercent = randomRangePercent;
      localProgressiveRatesEnabled = progressiveRatesEnabled;
      localTargetUploadRate = targetUploadRate;
      localTargetDownloadRate = targetDownloadRate;
      localProgressiveDurationHours = progressiveDurationHours;
      localPeerIdPattern = peerIdPattern;
      localKeyPattern = keyPattern;
      localScheduleSlowEnabled = scheduleSlowEnabled;
      localSlowStartLocal = utcToLocal(scheduleSlowStartUtc);
      localSlowEndLocal = utcToLocal(scheduleSlowEndUtc);
      localScheduleSlowScale = Math.round((scheduleSlowScale ?? 0.3) * 100);
    }
  });

  // 作息降速:UI 用本地小时显示,存储为 UTC 小时(换算随浏览器时区)
  const tzOffsetHours = -new Date().getTimezoneOffset() / 60;
  let localScheduleSlowEnabled = $state(false);
  let localSlowStartLocal = $state(23);
  let localSlowEndLocal = $state(8);
  let localScheduleSlowScale = $state(30);

  function localToUtc(hour) {
    return ((Math.round(hour) - tzOffsetHours) % 24 + 24) % 24;
  }
  function utcToLocal(hour) {
    return ((Math.round(hour) + tzOffsetHours) % 24 + 24) % 24;
  }
  function pushScheduleSlow() {
    if (isEditing) return;
    updateValues({
      scheduleSlowEnabled: localScheduleSlowEnabled,
      scheduleSlowStartUtc: Math.round(localToUtc(localSlowStartLocal)),
      scheduleSlowEndUtc: Math.round(localToUtc(localSlowEndLocal)),
      scheduleSlowScale: localScheduleSlowScale / 100,
    });
  }

  // 代理结构化字段:仅在实例的 proxyUrl 真正变化时回填(切换实例/应用/取消)。
  // 不放进上面的通用同步 effect,避免失焦等任何同步把正在输入的代理清空
  let lastProxyUrlFromInstance = $state(null);
  $effect(() => {
    if (proxyUrl !== lastProxyUrlFromInstance) {
      lastProxyUrlFromInstance = proxyUrl;
      localProxyUrl = proxyUrl;
      const parsed = parseProxyUrl(proxyUrl);
      proxyScheme = parsed.scheme;
      proxyHost = parsed.host;
      proxyPort = parsed.port;
      proxyUser = parsed.user;
      proxyPass = parsed.pass;
    }
  });

  $effect(() => {
    if (
      vpnPortSyncVisible &&
      localVpnPortSync &&
      vpnPortSyncEnabled &&
      currentForwardedPort &&
      localPort !== currentForwardedPort
    ) {
      localPort = currentForwardedPort;
    }
  });

  let networkStatusUnavailable = $derived(networkStatusError === 'unavailable');
  let vpnPortSyncBlocked = $derived(
    !networkStatusConfigured || !vpnPortSyncEnabled || networkStatusUnavailable
  );
  let useSyncedPort = $derived(
    vpnPortSyncVisible && localVpnPortSync && networkStatusConfigured && vpnPortSyncEnabled
  );
  let disableVpnPortSyncToggle = $derived(isRunning || (vpnPortSyncBlocked && !localVpnPortSync));

  // Helper to call onUpdate
  function updateValue(key, value) {
    if (onUpdate) {
      onUpdate({ [key]: value });
    }
  }

  function updateValues(values) {
    if (onUpdate) {
      onUpdate(values);
    }
  }

  // Validation constants
  const PORT_MIN = 1024;
  const PORT_MAX = 65535;
  const COMPLETION_MIN = 0;
  const COMPLETION_MAX = 100;
  const SCRAPE_INTERVAL_MIN = 10;
  const SCRAPE_INTERVAL_MAX = 3600;

  // Validate and sanitize port value
  function validatePort(value) {
    const parsed = parseInt(value, 10);
    if (isNaN(parsed) || parsed < PORT_MIN) {
      return PORT_MIN;
    }
    if (parsed > PORT_MAX) {
      return PORT_MAX;
    }
    return parsed;
  }

  // Validate and sanitize completion percent value
  function validateCompletionPercent(value) {
    const parsed = parseFloat(value);
    if (isNaN(parsed) || parsed < COMPLETION_MIN) {
      return COMPLETION_MIN;
    }
    if (parsed > COMPLETION_MAX) {
      return COMPLETION_MAX;
    }
    return parsed;
  }

  // Handle port input - only update if it's a valid number
  function handlePortInput() {
    const parsed = parseInt(localPort, 10);
    if (!isNaN(parsed)) {
      updateValue('port', parsed);
    }
  }

  // Handle port blur - validate and fix invalid values
  function handlePortBlur() {
    const validPort = validatePort(localPort);
    if (validPort !== localPort) {
      localPort = validPort;
      updateValue('port', validPort);
    }
    isEditing = false;
  }

  // Handle completion percent input
  function handleCompletionPercentInput() {
    const parsed = parseFloat(localCompletionPercent);
    if (!isNaN(parsed)) {
      updateValue('completionPercent', parsed);
    }
  }

  // Handle completion percent blur - validate and fix invalid values
  function handleCompletionPercentBlur() {
    const validPercent = validateCompletionPercent(localCompletionPercent);
    if (validPercent !== localCompletionPercent) {
      localCompletionPercent = validPercent;
      updateValue('completionPercent', validPercent);
    }
    isEditing = false;
  }

  function validateScrapeInterval(value) {
    const parsed = parseInt(value, 10);
    if (isNaN(parsed) || parsed < SCRAPE_INTERVAL_MIN) {
      return SCRAPE_INTERVAL_MIN;
    }
    if (parsed > SCRAPE_INTERVAL_MAX) {
      return SCRAPE_INTERVAL_MAX;
    }
    return parsed;
  }

  function handleScrapeIntervalInput() {
    const parsed = parseInt(localScrapeInterval, 10);
    if (!isNaN(parsed) && parsed >= SCRAPE_INTERVAL_MIN) {
      updateValue('scrapeInterval', parsed);
    }
  }

  function handleScrapeIntervalBlur() {
    const valid = validateScrapeInterval(localScrapeInterval);
    if (valid !== localScrapeInterval) {
      localScrapeInterval = valid;
      updateValue('scrapeInterval', valid);
    }
    isEditing = false;
  }

  // Focus/blur handlers to track editing state
  function handleFocus() {
    isEditing = true;
  }

  function handleBlur() {
    isEditing = false;
  }

  // 解析代理 URL: scheme://[user[:pass]@]host:port
  function parseProxyUrl(url) {
    const out = { scheme: 'socks5', host: '', port: '', user: '', pass: '' };
    if (!url || !url.includes('://')) return out;
    try {
      const u = new URL(url);
      out.scheme = u.protocol.replace(':', '') || 'socks5';
      out.host = u.hostname || '';
      out.port = u.port || '';
      out.user = u.username ? decodeURIComponent(u.username) : '';
      out.pass = u.password ? decodeURIComponent(u.password) : '';
    } catch {
      /* 解析失败保持字段为空 */
    }
    return out;
  }

  // 由结构化字段合成代理 URL 并写回实例
  function composeProxyUrl(scheme, host, port, user, pass) {
    if (!host || !port) return '';
    const auth = user
      ? `${encodeURIComponent(user)}${pass ? `:${encodeURIComponent(pass)}` : ''}@`
      : '';
    return `${scheme}://${auth}${host}:${port}`;
  }

  // 输入过程只更新本地预览;写回实例仅发生在「应用到当前实例 / 取消代理」按钮,
  // 避免逐字符触发配置同步,也杜绝同步回环把输入中的内容清空
  function handleProxyFieldChange() {
    localProxyUrl = composeProxyUrl(proxyScheme, proxyHost, proxyPort, proxyUser, proxyPass);
    proxyTestResult = null;
  }

  async function handleApplyProxyAll() {
    const composed = composeProxyUrl(proxyScheme, proxyHost, proxyPort, proxyUser, proxyPass);
    if (!composed) return;
    if (!window.confirm(`把代理 ${composed} 应用到全部实例?
(运行中的实例将跳过,下次启动生效)`)) return;
    try {
      const { api } = await import('$lib/api.js');
      const res = await api.applyProxyAll(composed);
      proxyTestResult = {
        ok: true,
        msg: `已应用到 ${res.updated} 个实例(跳过运行中 ${res.skippedRunning} 个)`,
      };
      api.frontendLog(
        'info',
        `代理应用到全部实例: ${composed},更新 ${res.updated},跳过 ${res.skippedRunning}`
      );
      // 同步前端实例列表,避免界面显示旧值
      instancesStore.update(list => list.map(i => ({ ...i, proxyUrl: composed })));
    } catch (e) {
      proxyTestResult = { ok: false, msg: `应用失败: ${e}` };
    }
  }

  async function handleProxyTest() {
    const composed = composeProxyUrl(proxyScheme, proxyHost, proxyPort, proxyUser, proxyPass);
    if (!composed) {
      proxyTestResult = { ok: false, msg: '请先填写地址与端口' };
      return;
    }
    proxyTesting = true;
    proxyTestResult = null;
    try {
      const { api } = await import('$lib/api.js');
      const ip = await api.testProxy(composed);
      proxyTestResult = { ok: true, msg: `测试成功,出口 IP: ${ip}` };
      api.frontendLog('info', `代理测试成功 ${composed} -> ${ip}`);
    } catch (e) {
      proxyTestResult = { ok: false, msg: `测试失败: ${e}` };
      api.frontendLog('warn', `代理测试失败 ${composed}: ${e}`);
    } finally {
      proxyTesting = false;
    }
  }

  // ===== 按站点绑定代理(防封:tracker 汇报 IP 与网站登录 IP 保持一致)=====
  const SITE_PROXY_KEY = 'rustatio-site-proxies';
  let showSiteBindings = $state(false);
  let siteBindings = $state([]); // [{ host, proxy }]

  function loadSiteBindings() {
    try {
      const saved = JSON.parse(localStorage.getItem(SITE_PROXY_KEY) || '{}');
      const hosts = new Set();
      for (const inst of get(instancesStore)) {
        const host = extractTrackerHost(inst?.torrent?.announce || '');
        if (host) hosts.add(host);
      }
      const savedHosts = Object.keys(saved);
      siteBindings = [...hosts, ...savedHosts.filter(h => !hosts.has(h))].map(h => ({
        host: h,
        proxy: saved[h] || '',
      }));
    } catch (e) {
      console.error('加载站点绑定失败:', e);
      siteBindings = [];
    }
  }

  function toggleSiteBindings() {
    showSiteBindings = !showSiteBindings;
    if (showSiteBindings) loadSiteBindings();
  }

  async function handleSiteBindingChange(binding, proxy) {
    binding.proxy = proxy;
    try {
      const saved = JSON.parse(localStorage.getItem(SITE_PROXY_KEY) || '{}');
      if (proxy) saved[binding.host] = proxy;
      else delete saved[binding.host];
      localStorage.setItem(SITE_PROXY_KEY, JSON.stringify(saved));
    } catch { /* ignore */ }
    const { api } = await import('$lib/api.js');
    let changed = 0;
    for (const inst of get(instancesStore)) {
      const host = extractTrackerHost(inst?.torrent?.announce || '');
      if (host !== binding.host || inst.isRunning) continue;
      instancesStore.update(list => list.map(i => (i.id === inst.id ? { ...i, proxyUrl: proxy } : i)));
      if (onUpdate) {
        // 空更新触发父级 updateInstance + syncConfigToServer,把新 proxyUrl 持久化到后端
        onUpdate({});
      }
      changed += 1;
    }
    proxyTestResult = {
      ok: true,
      msg: proxy
        ? `站点 ${binding.host} 已绑定代理 ${proxy}(${changed} 个实例生效)`
        : `站点 ${binding.host} 已改为直连(${changed} 个实例生效)`,
    };
    api.frontendLog('info', `站点代理绑定:${binding.host} -> ${proxy || '直连'},生效 ${changed} 个实例`);
  }

  // 已保存过的代理地址集合(供站点绑定下拉选择)
  let knownProxies = $derived.by(() => {
    const set = new Set();
    for (const b of siteBindings) if (b.proxy) set.add(b.proxy);
    if (localProxyUrl) set.add(localProxyUrl);
    return [...set];
  });

  // 当前实例是否正在使用代理(proxyUrl 即实例当前生效值)
  let instanceProxyInUse = $derived(Boolean(proxyUrl && String(proxyUrl).trim()));

  // 把当前填写的代理明确应用到当前选中实例
  async function handleApplyProxyCurrent() {
    const composed = composeProxyUrl(proxyScheme, proxyHost, proxyPort, proxyUser, proxyPass);
    if (!composed) {
      proxyTestResult = { ok: false, msg: '请先填写地址与端口' };
      return;
    }
    if (isRunning) {
      proxyTestResult = { ok: false, msg: '实例运行中无法修改配置,请先停止实例' };
      return;
    }
    const { api } = await import('$lib/api.js');
    updateValue('proxyUrl', composed);
    localProxyUrl = composed;
    proxyTestResult = { ok: true, msg: `已应用到当前实例: ${composed}` };
    api.frontendLog('info', `代理应用到当前实例: ${composed}`);
  }

  // 取消当前实例代理,恢复直连
  async function handleClearProxyCurrent() {
    if (isRunning) {
      proxyTestResult = { ok: false, msg: '实例运行中无法修改配置,请先停止实例' };
      return;
    }
    const { api } = await import('$lib/api.js');
    proxyHost = '';
    proxyPort = '';
    proxyUser = '';
    proxyPass = '';
    localProxyUrl = '';
    updateValue('proxyUrl', '');
    proxyTestResult = { ok: true, msg: '已取消当前实例代理,恢复直连' };
    api.frontendLog('info', '已取消当前实例代理,恢复直连');
  }

  // 状态模式切换:做种(100% 已完成)/ 下载中(可设完成度)
  function handleProgressModeChange(event) {
    if (isRunning) {
      event.target.value = localCompletionPercent >= 100 ? 'seeding' : 'downloading';
      return;
    }
    if (event.target.value === 'seeding') {
      localCompletionPercent = 100;
      updateValue('completionPercent', 100);
    } else {
      localCompletionPercent = 0;
      updateValue('completionPercent', 0);
    }
  }

  // 获取更多伪装档案:打开 .mRClient 档案的发布/搜索页面(在系统浏览器中打开)
  const isTauriForm = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  let showProfileSources = $state(false);
  const profileSources = [
    {
      name: 'SB Innovation 论坛 · mRatio Client Files',
      desc: '官方档案发布版块(需论坛账号,持续更新)',
      url: 'https://www.sb-innovation.de/forumdisplay.php?273-mRatio-Client-Files',
    },
    {
      name: 'SB Innovation 论坛首页',
      desc: 'mRatio 主站,注册后可下载全部档案',
      url: 'https://www.sb-innovation.de/',
    },
    {
      name: 'GitHub 代码搜索 "mRClient"',
      desc: '搜索全网托管在 GitHub 的 .mRClient 档案(需登录 GitHub)',
      url: 'https://github.com/search?q=mRClient&type=code',
    },
    {
      name: 'Google 搜索 ".mRClient 下载"',
      desc: '聚合搜索其它可能提供档案的站点',
      url: 'https://www.google.com/search?q=mRatio+.mRClient+download',
    },
    {
      name: 'Bing 搜索 ".mRClient 下载"',
      desc: '聚合搜索其它可能提供档案的站点',
      url: 'https://www.bing.com/search?q=mRatio+.mRClient+%E4%B8%8B%E8%BD%BD',
    },
  ];

  async function openProfileSource(url) {
    if (!url) return;
    if (isTauriForm) {
      try {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open(url);
      } catch (e) {
        console.error('打开链接失败:', e);
        window.open(url, '_blank', 'noopener');
      }
    } else {
      window.open(url, '_blank', 'noopener');
    }
  }

  function handleProfileSelect(event) {
    const profile = mrProfiles.find(x => x.fileName === event.target.value);
    if (profile) {
      updateValues({
        peerIdPattern: profile.peerIdPattern || '',
        keyPattern: profile.keyPattern || '',
        // mRatio announce 参数顺序模板与报告口径(档案里的 Announce/ReportUploadAs 等字段)
        announceQueryTemplate: profile.announceQueryTemplate || null,
        reportUploadAs: profile.reportUploadAs ?? 0,
        reportDownloadAs: profile.reportDownloadAs ?? 0,
        reportLeftAs: profile.reportLeftAs ?? 0,
      });
    }
    event.target.value = '';
  }

  function handleVpnPortSyncChange(checked) {
    if (checked && vpnPortSyncBlocked) {
      return;
    }

    localVpnPortSync = checked;

    if (checked && currentForwardedPort) {
      localPort = currentForwardedPort;
      updateValues({
        vpnPortSync: checked,
        port: currentForwardedPort,
      });
      return;
    }

    updateValue('vpnPortSync', checked);
  }
</script>

<Card class="p-3">
  <h2 class="mb-4 text-primary text-lg font-semibold flex items-center gap-2">
    <Settings size={20} /> 实例配置
  </h2>

  <!-- Client Settings -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <ClientIcon clientId={localSelectedClient} size={18} />
      <span class="text-sm font-medium">客户端</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border p-3">
      <div class="grid grid-cols-3 gap-3">
        <div>
          <Label for="client" class="text-xs text-muted-foreground mb-1.5 block">类型</Label>
          <ClientSelect
            {clients}
            bind:value={localSelectedClient}
            disabled={isRunning}
            onchange={() => updateValue('selectedClient', localSelectedClient)}
          />
        </div>
        <div>
          <Label for="clientVersion" class="text-xs text-muted-foreground mb-1.5 block"
            >版本</Label
          >
          <VersionSelect
            versions={clientVersions[localSelectedClient] || []}
            bind:value={localSelectedClientVersion}
            disabled={isRunning}
            onchange={() => updateValue('selectedClientVersion', localSelectedClientVersion)}
          />
        </div>
        <div>
          <div class="mb-1.5 flex items-center justify-between gap-2">
            <Label for="port" class="text-xs text-muted-foreground">端口</Label>
            {#if vpnPortSyncVisible}
              <div class="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                <Checkbox
                  id="vpn-port-sync"
                  bind:checked={localVpnPortSync}
                  disabled={disableVpnPortSyncToggle}
                  onchange={handleVpnPortSyncChange}
                />
                <Label for="vpn-port-sync" class="cursor-pointer flex items-center gap-1">
                  <Lock size={11} /> VPN 同步
                </Label>
                <InlineHelp text="可用时使用 Gluetun 当前的转发端口。" />
              </div>
            {/if}
          </div>
          <Input
            id="port"
            type="number"
            bind:value={localPort}
            disabled={isRunning || useSyncedPort}
            min="1024"
            max="65535"
            class={cn(
              'h-9 transition-colors',
              useSyncedPort &&
                'border-stat-upload/50 bg-stat-upload/10 text-stat-upload placeholder:text-stat-upload/60',
              useSyncedPort &&
                currentForwardedPort &&
                'ring-1 ring-stat-upload/30 focus-visible:ring-stat-upload'
            )}
            onfocus={handleFocus}
            onblur={handlePortBlur}
            oninput={handlePortInput}
          />
          {#if vpnPortSyncVisible && !networkStatusConfigured}
            <p class="mt-1 text-[11px] text-amber-400">未配置 VPN。</p>
          {:else if vpnPortSyncVisible && useSyncedPort && currentForwardedPort}
            <p class="mt-1 text-[11px] text-foreground/80">
              当前转发端口:<span class="font-mono">{currentForwardedPort}</span>
            </p>
          {:else if vpnPortSyncVisible && !vpnPortSyncEnabled && localVpnPortSync}
            <p class="mt-1 text-[11px] text-amber-400">
              服务器已禁用 VPN 同步。请取消勾选,或设置
              <span class="font-mono">VPN_PORT_SYNC=on</span> 后重启 Rustatio。
            </p>
          {:else if vpnPortSyncVisible && !vpnPortSyncEnabled}
            <p class="mt-1 text-[11px] text-amber-400">
              服务器已禁用 VPN 同步。设置 <span class="font-mono">VPN_PORT_SYNC=on</span
              >
              并重启 Rustatio 以启用。
            </p>
          {:else if vpnPortSyncVisible && networkStatusUnavailable}
            <p class="mt-1 text-[11px] text-amber-400">
              Gluetun 状态不可用。请确认 Gluetun 正在运行且已启用端口转发。
            </p>
          {:else if vpnPortSyncVisible && useSyncedPort && !currentForwardedPort}
            <p class="mt-1 text-[11px] text-amber-400">
              正在等待 Gluetun 的转发端口。请确认 <span class="font-mono"
                >VPN_PORT_FORWARDING=on</span
              >
              已启用且 VPN 服务商支持端口转发。
            </p>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Proxy(高亮独立区块) -->
  <div class="mb-4 rounded-lg border-2 border-primary/40 bg-primary/5 p-3">
    <div class="flex items-center gap-2 mb-1">
      <Globe size={16} class="text-primary" />
      <span class="text-sm font-semibold text-primary">代理设置(可选)</span>
      <InlineHelp text="为当前实例的 tracker 请求配置代理;留空直连" />
    </div>
    <p class="text-[11px] text-muted-foreground mb-3">
      支持 SOCKS5 / HTTP,可填用户名密码;填完点「测试代理」验证连通性(成功会显示出口 IP)
    </p>
      <div>
        <div class="grid grid-cols-[74px_1fr_84px] gap-2">
          <select
            class="h-9 rounded-md border border-border bg-card px-1.5 text-xs"
            bind:value={proxyScheme}
            disabled={isRunning}
            onchange={handleProxyFieldChange}
          >
            <option value="socks5">SOCKS5</option>
            <option value="http">HTTP</option>
          </select>
          <Input
            type="text"
            bind:value={proxyHost}
            disabled={isRunning}
            placeholder="地址 (如 127.0.0.1)"
            class="h-9 text-xs"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={handleProxyFieldChange}
          />
          <Input
            type="number"
            bind:value={proxyPort}
            disabled={isRunning}
            placeholder="端口"
            class="h-9 text-center text-xs"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={handleProxyFieldChange}
          />
        </div>
        <div class="mt-2 grid grid-cols-2 gap-2">
          <Input
            type="text"
            bind:value={proxyUser}
            disabled={isRunning}
            placeholder="用户名(可选)"
            class="h-9 text-xs"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={handleProxyFieldChange}
          />
          <Input
            type="password"
            bind:value={proxyPass}
            disabled={isRunning}
            placeholder="密码(可选)"
            class="h-9 text-xs"
            onfocus={handleFocus}
            onblur={handleBlur}
            oninput={handleProxyFieldChange}
          />
        </div>
        <div class="mt-2 flex flex-wrap items-center gap-2">
          <button
            type="button"
            class="h-8 rounded-md border border-border bg-muted/60 px-3 text-xs font-medium hover:bg-muted disabled:opacity-50"
            disabled={isRunning || proxyTesting}
            onclick={handleProxyTest}
          >
            {proxyTesting ? '测试中...' : '测试代理'}
          </button>
          <button
            type="button"
            class="h-8 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/90 disabled:opacity-50 shadow-sm"
            disabled={isRunning || proxyTesting}
            onclick={handleApplyProxyCurrent}
            title="把当前填写的代理应用到当前选中的实例"
          >
            应用到当前实例
          </button>
          <button
            type="button"
            class="h-8 rounded-md border border-primary/40 bg-primary/10 px-3 text-xs font-medium text-primary hover:bg-primary/20 disabled:opacity-50"
            disabled={isRunning || proxyTesting || !localProxyUrl}
            onclick={handleApplyProxyAll}
            title="把当前代理设置推送给所有实例(运行中的跳过)"
          >
            应用到全部实例
          </button>
          <button
            type="button"
            class="h-8 rounded-md border border-border bg-transparent px-3 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-50"
            disabled={isRunning || !instanceProxyInUse}
            onclick={handleClearProxyCurrent}
            title="取消当前实例的代理,恢复直连"
          >
            取消代理
          </button>
        </div>
        {#if proxyTestResult}
          <p
            class="mt-1.5 text-[11px] font-medium {proxyTestResult.ok
              ? 'text-stat-upload'
              : 'text-stat-download'}"
          >
            {proxyTestResult.ok ? '✓ ' : '✕ '}{proxyTestResult.msg}
          </p>
        {/if}
        <div
          class="mt-1.5 flex items-center gap-1.5 text-[11px] font-medium {instanceProxyInUse
            ? 'text-stat-upload'
            : 'text-muted-foreground'}"
        >
          {#if instanceProxyInUse}
            <CheckCircle2 size={12} />
            当前实例正在使用代理:{proxyUrl}
          {:else}
            <Ban size={12} />
            当前实例未使用代理(直连)
          {/if}
          <button
            type="button"
            class="ml-auto inline-flex items-center gap-1 text-[11px] font-medium text-primary hover:underline cursor-pointer bg-transparent border-0"
            onclick={toggleSiteBindings}
            title="为每个站点(tracker 域名)固定代理,让 tracker 汇报 IP 与网站登录 IP 一致"
          >
            按站点绑定代理 {showSiteBindings ? '▲' : '▼'}
          </button>
        </div>
        {#if showSiteBindings}
          <div class="mt-2 rounded-lg border border-border bg-muted/40 p-2 space-y-1.5">
            <p class="text-[10px] text-muted-foreground px-1">
              防封关键:同一站点,网页登录用的 IP 要和 tracker 汇报用的 IP 一致。给每个站点固定代理后,该站全部实例自动应用(运行中的实例需停止后重开生效)。
            </p>
            {#each siteBindings as binding (binding.host)}
              <div class="flex items-center gap-2 px-1">
                <span class="text-xs font-medium text-foreground w-44 truncate flex-shrink-0" title={binding.host}>
                  {binding.host}
                </span>
                <select
                  class="h-7 flex-1 rounded-md border border-border bg-card px-1.5 text-xs"
                  value={binding.proxy}
                  onchange={e => handleSiteBindingChange(binding, e.target.value)}
                >
                  <option value="">直连(不使用代理)</option>
                  {#each knownProxies as px (px)}
                    <option value={px}>{px}</option>
                  {/each}
                </select>
              </div>
            {/each}
            {#if siteBindings.length === 0}
              <p class="text-[11px] text-muted-foreground px-1">暂无站点(选择种子后自动列出 tracker 域名)</p>
            {/if}
          </div>
        {/if}
        {#if localProxyUrl}
          <p class="mt-1 truncate font-mono text-[10px] text-muted-foreground">
            {localProxyUrl}
          </p>
        {/if}
      </div>
  </div>

  <!-- Transfer Rates -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <ArrowUpDown size={16} class="text-muted-foreground" />
      <span class="text-sm font-medium">传输速率</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
      <div class="grid grid-cols-2">
        <div class="p-3 border-r border-border">
          <div class="flex items-center gap-2 mb-2">
            <Upload size={14} class="text-stat-upload" />
            <span class="text-xs text-muted-foreground">上传</span>
          </div>
          <div class="flex items-center gap-2">
            <Input
              id="upload"
              type="number"
              bind:value={localUploadRate}
              disabled={isRunning}
              min="0"
              step="0.1"
              class="flex-1 h-9 text-center font-medium"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('uploadRate', localUploadRate)}
            />
            <span class="text-sm text-muted-foreground">KB/s</span>
          </div>
        </div>
        <div class="p-3">
          <div class="flex items-center gap-2 mb-2">
            <Download size={14} class="text-stat-download" />
            <span class="text-xs text-muted-foreground">下载</span>
          </div>
          <div class="flex items-center gap-2">
            <Input
              id="download"
              type="number"
              bind:value={localDownloadRate}
              disabled={isRunning}
              min="0"
              step="0.1"
              class="flex-1 h-9 text-center font-medium"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('downloadRate', localDownloadRate)}
            />
            <span class="text-sm text-muted-foreground">KB/s</span>
          </div>
          {#if localDownloadRate > 0 && localCompletionPercent >= 100}
            <p class="text-[10px] text-orange-500 mt-1">已完成 100% 时无效果</p>
          {/if}
        </div>
      </div>
    </div>
  </div>

  <!-- Initial State -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <Clock size={16} class="text-muted-foreground" />
      <span class="text-sm font-medium">初始状态</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border p-3">
      <div class="mb-3">
        <Label for="progressMode" class="text-xs text-muted-foreground mb-1.5 block"
          >状态模式</Label
        >
        <select
          id="progressMode"
          class="w-full h-9 rounded-md border border-border bg-card px-2 text-xs"
          disabled={isRunning}
          value={localCompletionPercent >= 100 ? 'seeding' : 'downloading'}
          onchange={handleProgressModeChange}
        >
          <option value="seeding">做种 — 种子已下载完成,纯做种(完成度 100%)</option>
          <option value="downloading">下载中 — 可自定义完成度</option>
        </select>
      </div>
      <div class="grid grid-cols-2 gap-3">
        <div>
          <Label for="completion" class="text-xs text-muted-foreground mb-1.5 block"
            >完成度</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="completion"
              type="number"
              bind:value={localCompletionPercent}
              disabled={isRunning || localCompletionPercent >= 100}
              min="0"
              max="100"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleCompletionPercentBlur}
              oninput={handleCompletionPercentInput}
            />
            <span class="text-sm text-muted-foreground">%</span>
          </div>
          {#if localCompletionPercent >= 100}
            <p class="text-[10px] text-stat-upload mt-1">做种模式:Tracker 视为完整做种</p>
          {/if}
        </div>
        <div>
          <Label for="initialUp" class="text-xs text-muted-foreground mb-1.5 block"
            >已上传量</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="initialUp"
              type="number"
              bind:value={localInitialUploaded}
              disabled={isRunning}
              min="0"
              step="1"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('initialUploaded', Math.round(localInitialUploaded || 0))}
            />
            <span class="text-sm text-muted-foreground">MB</span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Timing -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <Timer size={16} class="text-muted-foreground" />
      <span class="text-sm font-medium">时间设置</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border p-3">
      <div class="grid grid-cols-2 gap-3">
        <div>
          <Label for="updateInterval" class="text-xs text-muted-foreground mb-1.5 block"
            >刷新间隔</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="updateInterval"
              type="number"
              bind:value={localUpdateIntervalSeconds}
              disabled={isRunning}
              min="1"
              max="300"
              step="1"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleBlur}
              oninput={() => updateValue('updateIntervalSeconds', localUpdateIntervalSeconds)}
            />
            <span class="text-sm text-muted-foreground">秒</span>
          </div>
        </div>
        <div>
          <Label for="scrapeInterval" class="text-xs text-muted-foreground mb-1.5 block"
            >Scrape 间隔</Label
          >
          <div class="flex items-center gap-2">
            <Input
              id="scrapeInterval"
              type="number"
              bind:value={localScrapeInterval}
              disabled={isRunning}
              min="10"
              max="3600"
              step="1"
              class="flex-1 h-9 text-center"
              onfocus={handleFocus}
              onblur={handleScrapeIntervalBlur}
              oninput={handleScrapeIntervalInput}
            />
            <span class="text-sm text-muted-foreground">秒</span>
          </div>
        </div>
      </div>

      <!-- 作息降速(防封) -->
      <div class="border-t border-border pt-3">
        <label class="flex items-center gap-2 cursor-pointer mb-2">
          <input
            type="checkbox"
            bind:checked={localScheduleSlowEnabled}
            disabled={isRunning}
            class="w-4 h-4 rounded border-border text-primary focus:ring-primary/50"
            onchange={pushScheduleSlow}
          />
          <span class="text-xs font-medium">作息降速(模拟真人夜间挂机,防封)</span>
        </label>
        {#if localScheduleSlowEnabled}
          <div class="grid grid-cols-[auto_1fr_1fr_1fr] items-end gap-2">
            <div>
              <Label class="text-xs text-muted-foreground mb-1.5 block">开始(本地)</Label>
              <Input
                type="number"
                bind:value={localSlowStartLocal}
                disabled={isRunning}
                min="0"
                max="23"
                class="h-9 text-center"
                onfocus={handleFocus}
                onblur={handleBlur}
                onchange={pushScheduleSlow}
              />
            </div>
            <div>
              <Label class="text-xs text-muted-foreground mb-1.5 block">结束(本地)</Label>
              <Input
                type="number"
                bind:value={localSlowEndLocal}
                disabled={isRunning}
                min="0"
                max="23"
                class="h-9 text-center"
                onfocus={handleFocus}
                onblur={handleBlur}
                onchange={pushScheduleSlow}
              />
            </div>
            <div>
              <Label class="text-xs text-muted-foreground mb-1.5 block">时段速率</Label>
              <div class="flex items-center gap-1">
                <Input
                  type="number"
                  bind:value={localScheduleSlowScale}
                  disabled={isRunning}
                  min="0"
                  max="100"
                  class="flex-1 h-9 text-center"
                  onfocus={handleFocus}
                  onblur={handleBlur}
                  onchange={pushScheduleSlow}
                />
                <span class="text-xs text-muted-foreground">%</span>
              </div>
            </div>
          </div>
          <p class="mt-1.5 text-[10px] text-muted-foreground">
            时段内上传/下载速率降到设定比例(默认 23:00 - 08:00 降到 30%),支持跨午夜;随软件时区自动换算。
          </p>
        {/if}
      </div>
    </div>
  </div>

  <!-- Advanced (mRatio) -->
  <div class="mb-4">
    <div class="flex items-center gap-2 mb-3">
      <Fingerprint size={16} class="text-muted-foreground" />
      <span class="text-sm font-medium">高级伪装(mRatio)</span>
    </div>
    <div class="bg-muted/50 rounded-lg border border-border p-3 space-y-3">
      <div>
        <div class="mb-1.5 flex items-center gap-2">
          <Label for="peerIdPattern" class="text-xs text-muted-foreground">peer_id 正则模板</Label>
          <InlineHelp text="按正则模板生成 peer_id,须恰好 20 个 ASCII 字符,留空用内置生成。例:-qB5230-[\\w]{12}" />
        </div>
        <Input
          id="peerIdPattern"
          type="text"
          bind:value={localPeerIdPattern}
          disabled={isRunning}
          placeholder="-qB5230-[\w]{12}"
          class="h-9 font-mono text-xs"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('peerIdPattern', localPeerIdPattern)}
        />
      </div>
      <div>
        <div class="mb-1.5 flex items-center gap-2">
          <Label for="keyPattern" class="text-xs text-muted-foreground">key 正则模板</Label>
          <InlineHelp text="按正则模板生成 announce 的 key 参数,留空用内置 8 位十六进制。例:[0-9a-f]{8}" />
        </div>
        <Input
          id="keyPattern"
          type="text"
          bind:value={localKeyPattern}
          disabled={isRunning}
          placeholder="[0-9a-f]{8}"
          class="h-9 font-mono text-xs"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('keyPattern', localKeyPattern)}
        />
      </div>

      <div>
        <div class="mb-1.5 flex items-center gap-2">
          <Label class="text-xs text-muted-foreground">伪装档案(mRatioClients)</Label>
          <InlineHelp text="选择 .mRClient 档案,自动填充下方 peer_id/key 模板;也可手动填写" />
          <button
            type="button"
            class="ml-auto inline-flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2 py-1 text-[11px] font-medium text-primary hover:bg-primary/20 transition-colors cursor-pointer"
            onclick={() => (showProfileSources = !showProfileSources)}
            title="下载别人做好的 .mRClient 伪装档案"
          >
            <Download size={12} />获取档案<ChevronDown size={11} />
          </button>
        </div>
        {#if showProfileSources}
          <div class="mb-2 rounded-lg border border-border bg-muted/40 p-2 space-y-1">
            <p class="text-[10px] text-muted-foreground px-1 pb-1">
              档案放到软件旁的 mRatioClients 文件夹即可自动加载(下载后无需重启,重新打开软件生效)
            </p>
            {#each profileSources as src (src.url)}
              <button
                type="button"
                class="w-full flex items-start gap-2 rounded-md px-2 py-1.5 text-left hover:bg-muted transition-colors cursor-pointer bg-transparent border-0"
                onclick={() => openProfileSource(src.url)}
              >
                <ExternalLink size={13} class="mt-0.5 flex-shrink-0 text-primary" />
                <span class="min-w-0">
                  <span class="block text-xs font-medium text-foreground">{src.name}</span>
                  <span class="block text-[10px] text-muted-foreground">{src.desc}</span>
                </span>
              </button>
            {/each}
          </div>
        {/if}
        <select
          class="h-9 w-full rounded-md border border-border bg-card px-2 text-xs"
          value=""
          onchange={handleProfileSelect}
          disabled={isRunning || mrProfiles.length === 0}
        >
          <option value="">
            {mrProfiles.length === 0 ? '未找到 mRatioClients 目录' : `手动填写(共 ${mrProfiles.length} 个档案)`}
          </option>
          {#each mrProfiles as p (p.fileName)}
            <option value={p.fileName}>{p.name} — {p.author}</option>
          {/each}
        </select>
      </div>
      <p class="text-[10px] text-muted-foreground">
        以上功能移植自 mRatio:模板精确复刻客户端 peer_id 结构,代理支持逐实例配置。
      </p>
    </div>
  </div>

  <!-- Randomization -->
  <div class="mb-3">
    <RandomizationSettings
      bind:enabled={localRandomizeRates}
      bind:rangePercent={localRandomRangePercent}
      uploadRate={localUploadRate}
      downloadRate={localDownloadRate}
      disabled={isRunning}
      onchange={updates => {
        for (const [key, value] of Object.entries(updates)) updateValue(key, value);
      }}
    />
  </div>

  <!-- Progressive Rates -->
  <div class="mb-0">
    <ProgressiveRateSettings
      bind:enabled={localProgressiveRatesEnabled}
      bind:durationHours={localProgressiveDurationHours}
      bind:targetUploadRate={localTargetUploadRate}
      bind:targetDownloadRate={localTargetDownloadRate}
      uploadRate={localUploadRate}
      downloadRate={localDownloadRate}
      disabled={isRunning}
      onchange={updates => {
        for (const [key, value] of Object.entries(updates)) updateValue(key, value);
      }}
    />
  </div>
</Card>
