# -*- coding: utf-8 -*-
"""前端补丁:api.js 新方法 + ConfigurationForm 结构化代理与测试按钮。"""
import io

# ============ 1. api.js ============
p = 'ui/src/lib/api.js'
s = io.open(p, encoding='utf-8').read()
old = """  reattachMetadataTorrent: async (id, record) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('reattach_metadata_torrent', { instanceId: Number(id), record });
  },"""
new = """  reattachMetadataTorrent: async (id, record) => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('reattach_metadata_torrent', { instanceId: Number(id), record });
  },
  testProxy: async proxyUrl => {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke('test_proxy', { proxyUrl });
  },
  frontendLog: async (level, message) => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('frontend_log', { level, message });
    } catch {
      /* 日志失败不影响功能 */
    }
  },"""
assert s.count(old) == 1, 'api.js anchor'
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('api.js ok')

# ============ 2. ConfigurationForm ============
p = 'ui/src/components/config/ConfigurationForm.svelte'
s = io.open(p, encoding='utf-8').read()

old = """  let localPeerIdPattern = $state('');
  let localKeyPattern = $state('');
  let localProxyUrl = $state('');"""
new = """  let localPeerIdPattern = $state('');
  let localKeyPattern = $state('');
  let localProxyUrl = $state('');
  // 结构化代理输入(协议/地址/端口/用户名/密码)
  let proxyScheme = $state('socks5');
  let proxyHost = $state('');
  let proxyPort = $state('');
  let proxyUser = $state('');
  let proxyPass = $state('');
  let proxyTestResult = $state(null); // { ok, msg }
  let proxyTesting = $state(false);"""
assert s.count(old) == 1, 'form state anchor'
s = s.replace(old, new)

old = """      localPeerIdPattern = peerIdPattern;
      localKeyPattern = keyPattern;
      localProxyUrl = proxyUrl;
    }
  });"""
new = """      localPeerIdPattern = peerIdPattern;
      localKeyPattern = keyPattern;
      localProxyUrl = proxyUrl;
      // 解析已有代理 URL 到结构化字段
      const parsed = parseProxyUrl(proxyUrl);
      proxyScheme = parsed.scheme;
      proxyHost = parsed.host;
      proxyPort = parsed.port;
      proxyUser = parsed.user;
      proxyPass = parsed.pass;
    }
  });"""
assert s.count(old) == 1, 'form effect anchor'
s = s.replace(old, new)

old = """  function handleProfileSelect(event) {"""
new = """  // 解析代理 URL: scheme://[user[:pass]@]host:port
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

  function handleProxyFieldChange() {
    const composed = composeProxyUrl(proxyScheme, proxyHost, proxyPort, proxyUser, proxyPass);
    localProxyUrl = composed;
    proxyTestResult = null;
    updateValue('proxyUrl', composed);
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

  function handleProfileSelect(event) {"""
assert s.count(old) == 1, 'form handler anchor'
s = s.replace(old, new)

old = """      <div>
        <div class="mb-1.5 flex items-center gap-2">
          <Globe size={12} class="text-muted-foreground" />
          <Label for="proxyUrl" class="text-xs text-muted-foreground">代理地址</Label>
          <InlineHelp text="tracker 请求走代理,支持 http(s):// 与 socks5://,留空直连。" />
        </div>
        <Input
          id="proxyUrl"
          type="text"
          bind:value={localProxyUrl}
          disabled={isRunning}
          placeholder="socks5://127.0.0.1:1080"
          class="h-9 font-mono text-xs"
          onfocus={handleFocus}
          onblur={handleBlur}
          oninput={() => updateValue('proxyUrl', localProxyUrl)}
        />
      </div>"""
new = """      <div>
        <div class="mb-1.5 flex items-center gap-2">
          <Globe size={12} class="text-muted-foreground" />
          <Label class="text-xs text-muted-foreground">代理设置</Label>
          <InlineHelp text="tracker 请求走代理(http/socks5),留空直连;填完可点击测试验证连通性" />
        </div>
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
        <div class="mt-2 flex items-center gap-2">
          <button
            type="button"
            class="h-8 rounded-md border border-border bg-muted/60 px-3 text-xs font-medium hover:bg-muted disabled:opacity-50"
            disabled={isRunning || proxyTesting}
            onclick={handleProxyTest}
          >
            {proxyTesting ? '测试中...' : '测试代理'}
          </button>
          {#if proxyTestResult}
            <span
              class="text-[11px] {proxyTestResult.ok
                ? 'text-stat-upload'
                : 'text-stat-download'}"
            >
              {proxyTestResult.msg}
            </span>
          {/if}
        </div>
        {#if localProxyUrl}
          <p class="mt-1.5 truncate font-mono text-[10px] text-muted-foreground">
            {localProxyUrl}
          </p>
        {/if}
      </div>"""
assert s.count(old) == 1, 'form proxy ui anchor'
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='\n').write(s)
print('ConfigurationForm ok')
print('ALL OK')
