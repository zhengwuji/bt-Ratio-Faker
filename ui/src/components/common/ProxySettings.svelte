<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { getProxyUrl, setProxyUrl, getRunMode } from '$lib/api.js';
  import { Globe, Save, Trash2, AlertTriangle, CheckCircle } from '@lucide/svelte';

  // Only show proxy settings in WASM mode (GitHub Pages)
  // Desktop (Tauri) and Server modes don't have CORS limitations
  let runMode = $derived(getRunMode());

  let proxyUrl = $state(getProxyUrl());
  let showHelp = $state(false);

  function saveProxy() {
    setProxyUrl(proxyUrl);
    alert('代理 URL 已保存!刷新页面后生效。');
  }

  function clearProxy() {
    proxyUrl = '';
    setProxyUrl('');
    alert('代理已清除!刷新页面后生效。');
  }
</script>

<!-- Only show in WASM mode (GitHub Pages) - Desktop and Server don't need CORS proxy -->
{#if runMode === 'wasm'}
  <Card class="p-3 mb-3">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-primary text-lg font-semibold flex items-center gap-2">
        <Globe size={20} />CORS 代理(可选)</h2>
      <button
        class="text-muted-foreground hover:text-foreground text-sm"
        onclick={() => (showHelp = !showHelp)}
      >
        {showHelp ? '▼ 收起帮助' : '▶ 展开帮助'}
      </button>
    </div>

    {#if showHelp}
      <div class="bg-muted/50 p-3 rounded-lg mb-3 text-sm">
        <p class="mb-2">
          <strong>为什么需要这个?</strong> 多数 BitTorrent tracker 不支持 CORS,浏览器因此无法直接请求 them.
        </p>
        <p class="mb-2">
          <strong>方案 1(推荐):</strong>使用<a
            href="https://github.com/zhengwuji/bt-Ratio-Faker/releases/latest"
            target="_blank"
            class="text-primary hover:underline font-semibold"
          >桌面应用</a>
          桌面版没有 CORS 限制,可直接连接所有 tracker。
        </p>
        <p class="mb-2">
          <strong>方案 2:</strong>部署免费的 Cloudflare Worker 作为 CORS 代理,参见<a
            href="https://github.com/zhengwuji/bt-Ratio-Faker/blob/main/WEB_VERSION.md"
            target="_blank"
            class="text-primary hover:underline"
          >配置指南</a>获取分步说明(约 5 分钟)。</p>
        <p class="mb-2">
          <strong>Worker URL 示例:</strong>
          <code class="bg-background px-2 py-1 rounded text-xs">
            https://rustatio-cors-proxy.yourname.workers.dev
          </code>
        </p>
        <p class="text-stat-danger flex items-center gap-1.5">
          <AlertTriangle size={16} class="flex-shrink-0" /> 未配置代理时,只能连接支持 CORS 的 tracker。
        </p>
      </div>
    {/if}

    <div class="flex flex-col gap-2">
      <label for="proxy-url" class="text-sm font-medium">代理 URL(留空禁用)</label>
      <input
        id="proxy-url"
        type="url"
        bind:value={proxyUrl}
        placeholder="https://your-worker.workers.dev"
        class="w-full px-3 py-2 bg-background border border-border rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary"
      />
      <div class="flex gap-2">
        <Button onclick={saveProxy} class="flex-1">
          {#snippet children()}
            <span class="flex items-center gap-1.5"><Save size={16} />保存代理</span>
          {/snippet}
        </Button>
        {#if proxyUrl}
          <Button
            onclick={clearProxy}
            class="flex-1 bg-stat-danger hover:bg-stat-danger/90 text-white shadow-sm"
          >
            {#snippet children()}
              <span class="flex items-center gap-1.5"><Trash2 size={16} />清除</span>
            {/snippet}
          </Button>
        {/if}
      </div>
      {#if proxyUrl}
        <p class="text-xs text-stat-upload flex items-center gap-1.5">
          <CheckCircle size={14} class="flex-shrink-0" /> 已配置代理:所有 tracker 请求都会经由该代理转发
        </p>
      {:else}
        <p class="text-xs text-stat-ratio flex items-center gap-1.5">
          <AlertTriangle size={14} class="flex-shrink-0" /> 未配置代理:只能连接支持 CORS 的 tracker
        </p>
      {/if}
    </div>
  </Card>
{/if}
