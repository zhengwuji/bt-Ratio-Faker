<script>
  import BaseModal from '../common/BaseModal.svelte';
  import { Users, Globe } from '@lucide/svelte';

  let { isOpen = $bindable(false), peers = [], stats = null, myIp = '' } = $props();

  // IP -> 国家(打开时经后端批量查询 ip-api.com;结果缓存)
  let geoMap = $state({});
  let geoLoading = $state(false);

  function flagOf(code) {
    if (!code) return '🏳️';
    return String.fromCodePoint(
      ...code
        .toUpperCase()
        .split('')
        .map(c => 0x1f1e6 + c.charCodeAt(0) - 65)
    );
  }

  async function loadGeo() {
    if (!peers?.length || geoLoading) return;
    const ips = [...new Set(peers.map(p => p.ip))].filter(Boolean);
    const missing = ips.filter(ip => !(ip in geoMap));
    if (missing.length === 0) return;
    geoLoading = true;
    try {
      const { api } = await import('$lib/api.js');
      const results = await api.geoLookupIps(missing);
      const next = { ...geoMap };
      for (const r of results || []) {
        next[r.ip] = { country: r.country || '', code: r.countryCode || '' };
      }
      for (const ip of missing) {
        if (!next[ip]) next[ip] = { country: '', code: '' };
      }
      geoMap = next;
    } catch (e) {
      console.error('IP 归属地查询失败:', e);
    } finally {
      geoLoading = false;
    }
  }

  $effect(() => {
    if (isOpen) loadGeo();
  });

  function isSelf(peer) {
    return myIp && peer.ip === myIp;
  }

  const CLIENT_PREFIXES = [
    ['-qB', 'qBittorrent'],
    ['-UT', 'µTorrent'],
    ['-BT', 'BitTorrent'],
    ['-DT', 'DT (Download STudio)'],
    ['-TR', 'Transmission'],
    ['-DE', 'Deluge'],
    ['-LT', 'libtorrent'],
    ['-FG', 'Freebox'],
    ['-XX', 'Xunlei'],
    ['-SD', 'Thunder'],
    ['-XL', 'Xunlei'],
    ['M7-', 'Xunlei'],
    ['-ML', 'MLdonkey'],
    ['-TT', 'TuoTu'],
    ['-QQ', 'Xunlei QQ'],
    ['-NX', 'Net Transport'],
    ['-TS', 'TorrentStorm'],
    ['A-', 'Ares'],
    ['S577', 'Shad0w'],
    ['Mbrst', 'Burst'],
];

  // 从 peer_id 前缀解析客户端名称;qBittorrent 等主流客户端可精确识别
  function clientName(peer) {
    if (!peer.peer_id) return '—';
    for (const [prefix, name] of CLIENT_PREFIXES) {
      if (peer.peer_id.startsWith(prefix)) {
        const ver = peer.peer_id.slice(prefix.length, prefix.length + 3);
        const version = ver
          ? ` ${ver
              .split('')
              .map(c => (/\d/.test(c) ? c : ''))
              .join('')}`.trimEnd()
          : '';
        return name + (version ? ` ${version}` : '');
      }
    }
    return '未知客户端';
  }

  let selfIp = $derived('');
</script>

{#if isOpen}
  <BaseModal
    bind:open={isOpen}
    onClose={() => (isOpen = false)}
    titleId="peers-dialog-title"
    maxWidthClass="max-w-xl"
    panelClass="max-h-[80vh] flex flex-col animate-in fade-in zoom-in-95 duration-200"
  >
    <div class="flex items-center justify-between border-b border-border p-4">
      <div class="flex items-center gap-2">
        <Users size={18} class="text-primary" />
        <h3 id="peers-dialog-title" class="text-base font-semibold text-foreground">
          正在做种的 Peer 列表
        </h3>
      </div>
      <button
        class="rounded p-1 hover:bg-muted cursor-pointer bg-transparent border-0"
        onclick={() => (isOpen = false)}
        aria-label="关闭"
      >
        ✕
      </button>
    </div>

    <div class="border-b border-border bg-muted/30 px-4 py-2 text-xs text-muted-foreground">
      数据来自最近一次 tracker 汇报响应(numwant 上限 50)。{#if myIp}本机出口 IP:{myIp},表中已高亮。{/if}
      {#if stats}
        当前统计:做种 {stats.seeders ?? 0} / 下载 {stats.leechers ?? 0}
      {/if}
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      {#if !peers || peers.length === 0}
        <p class="text-sm text-muted-foreground italic">
          暂无 peer 数据 —— 开始运行并完成一次 tracker 汇报后,这里会显示 tracker 返回的参与 peer。
        </p>
      {:else}
        <div class="overflow-hidden rounded-lg border border-border">
          <table class="w-full text-xs">
            <thead>
              <tr class="bg-muted/60 text-muted-foreground">
                <th class="px-3 py-2 text-left font-medium">#</th>
                <th class="px-3 py-2 text-left font-medium">IP 地址</th>
                <th class="px-3 py-2 text-left font-medium">国家/地区</th>
                <th class="px-3 py-2 text-left font-medium">端口</th>
                <th class="px-3 py-2 text-left font-medium">客户端</th>
              </tr>
            </thead>
            <tbody>
              {#each peers as peer, i (peer.ip + ':' + peer.port + '-' + i)}
                <tr class="border-t border-border hover:bg-muted/40 {isSelf(peer) ? 'bg-primary/10' : ''}">
                  <td class="px-3 py-1.5 text-muted-foreground">{i + 1}</td>
                  <td class="px-3 py-1.5 font-mono break-all">
                    <span class="inline-flex items-center gap-1">
                      <Globe size={11} class="text-muted-foreground flex-shrink-0" />{peer.ip}
                      {#if isSelf(peer)}
                        <span
                          class="ml-1 rounded bg-primary/20 px-1.5 py-0.5 text-[10px] font-semibold text-primary"
                        >本机</span>
                      {/if}
                    </span>
                  </td>
                  <td class="px-3 py-1.5 whitespace-nowrap">
                    {#if geoLoading && !(peer.ip in geoMap)}
                      <span class="text-muted-foreground">…</span>
                    {:else if geoMap[peer.ip]?.country}
                      <span>{flagOf(geoMap[peer.ip].code)} {geoMap[peer.ip].country}</span>
                    {:else}
                      <span class="text-muted-foreground">—</span>
                    {/if}
                  </td>
                  <td class="px-3 py-1.5 font-mono text-muted-foreground">{peer.port}</td>
                  <td class="px-3 py-1.5">
                    {#if peer.peer_id}
                      {clientName(peer)}
                    {:else}
                      <span class="text-muted-foreground" title="tracker 以 compact 模式返回,不含客户端 ID">—</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="mt-2 text-[10px] text-muted-foreground italic">
          共 {peers.length} 个 peer。IP 为对方与 tracker 通信的地址,可能与对方真实出口不同(NAT/代理)。
          客户端一列仅当 tracker 返回非 compact 响应(带 peer id)时有值;当前请求保持 qBittorrent
          真实指纹(compact 模式),tracker 不会返回对方客户端。
        </p>
      {/if}
    </div>
  </BaseModal>
{/if}
