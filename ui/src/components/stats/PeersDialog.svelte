<script>
  import BaseModal from '../common/BaseModal.svelte';
  import { Users, Globe, Radar } from '@lucide/svelte';

  let {
    isOpen = $bindable(false),
    peers = [],
    stats = null,
    myIp = '',
    instanceId = null,
  } = $props();

  // IP -> 国家(打开时经后端批量查询 ip-api.com;结果缓存)
  let geoMap = $state({});
  let geoLoading = $state(false);

  // 'ip:port' -> { online, peerId }(直连 BT 握手探测结果;手动触发后缓存)
  let probeMap = $state({});
  let probing = $state(false);
  let lastOpenedFor = null;

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
    if (!isOpen) return;
    // 切换了实例时清空缓存,避免把上一个实例的数据串过来
    if (lastOpenedFor !== String(instanceId)) {
      geoMap = {};
      probeMap = {};
      lastOpenedFor = String(instanceId);
    }
    loadGeo();
  });

  function isSelf(peer) {
    return myIp && peer.ip === myIp;
  }

  // 同一 ip:port 只保留一行(tracker 可能把同一 peer 以多地址族/重复形式返回)
  let uniquePeers = $derived.by(() => {
    const seen = new Set();
    const out = [];
    for (const p of peers || []) {
      const key = `${p.ip}:${p.port}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(p);
    }
    return out;
  });

  // 直连 BT 握手探测对方客户端:使用本实例的 info_hash 与伪装 peer_id,
  // 与 tracker 汇报指纹一致;握手后立即断开,tracker 不感知,无账号风险。
  // 本机行不探测。
  async function probeClients() {
    if (probing || !instanceId || !uniquePeers.length) return;
    const targets = uniquePeers.filter(p => !isSelf(p)).map(p => ({ ip: p.ip, port: p.port }));
    if (!targets.length) return;
    probing = true;
    try {
      const { api } = await import('$lib/api.js');
      const results = await api.probePeerClients(instanceId, targets);
      const next = { ...probeMap };
      for (const r of results || []) {
        next[`${r.ip}:${r.port}`] = { online: !!r.online, peerId: r.peer_id || '' };
      }
      probeMap = next;
    } catch (e) {
      console.error('客户端探测失败:', e);
    } finally {
      probing = false;
    }
  }

  // BT 标准 peer_id 编码:BEP 20 两位前缀(如 -qB5030-)+ Azureus 风格 + 旧式前缀
  const CLIENT_NAMES = {
    qB: 'qBittorrent',
    TR: 'Transmission',
    DE: 'Deluge',
    LT: 'libtorrent',
    UM: 'µTorrent Mac',
    UT: 'µTorrent',
    BT: 'BitTorrent',
    BC: 'BitComet',
    XL: '迅雷 Xunlei',
    SD: '迅雷 Thunder',
    XX: '迅雷 Xunlei',
    TT: '拓土 TuoTu',
    QQ: '迅雷 QQ',
    QD: 'QQDownload',
    A2: 'aria2',
    FD: 'Free Download Manager',
    FG: 'FlashGet',
    FB: 'Freebox',
    DT: 'DownloadStudio',
    PI: 'PicoTorrent',
    BB: 'BiglyBT',
    WW: 'WebTorrent',
    AZ: 'Azureus',
    ML: 'MLdonkey',
    TS: 'TorrentStorm',
    NX: 'Net Transport',
  };

  const LEGACY_PREFIXES = [
    ['M7-', '迅雷 Xunlei'],
    ['exbc', 'BitComet (旧版)'],
    ['Plus', 'BitComet Plus'],
    ['S577', 'Shad0w'],
    ['Mbrst', 'Burst'],
    ['A-', 'Ares'],
    ['XBT', 'XBT Client'],
  ];

  function cleanPeerId(pid) {
    return String(pid || '')
      .replace(/[^\x20-\x7e]/g, '')
      .trim();
  }

  function clientFromPeerId(pid) {
    const id = cleanPeerId(pid);
    if (!id) return null;
    const m = id.match(/^-(..)(\d{3,4})-/);
    if (m && CLIENT_NAMES[m[1]]) {
      return `${CLIENT_NAMES[m[1]]} ${m[2].slice(0, 3).split('').join('.')}`;
    }
    const az = id.match(/^M(\d)-(\d)-(\d)-/); // Azureus/Vuze 风格 M5-0-7--
    if (az) return `Vuze ${az[1]}.${az[2]}.${az[3]}`;
    const bt = id.match(/^T(\d)(\d)([A-Za-z])/); // BitTornado T03C---
    if (bt) return `BitTornado ${bt[1]}.${bt[2]}${bt[3]}`;
    const sh = id.match(/^S(\d)(\d)(\d)/); // Shadow 风格 S577-
    if (sh) return `Shad0w ${sh[1]}.${sh[2]}.${sh[3]}`;
    for (const [prefix, name] of LEGACY_PREFIXES) {
      if (id.startsWith(prefix)) return name;
    }
    return '未知客户端';
  }

  function clientCell(peer) {
    const probe = probeMap[`${peer.ip}:${peer.port}`];
    const rawId = cleanPeerId(peer.peer_id) || cleanPeerId(probe?.peerId);
    return {
      name: rawId ? clientFromPeerId(rawId) : null,
      rawId,
      online: probe ? probe.online : null,
    };
  }
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

    <div class="flex items-center justify-between gap-3 border-b border-border bg-muted/20 px-4 py-2">
      <p class="text-[11px] leading-snug text-muted-foreground">
        tracker 的 compact 响应只含 IP/端口。点「探测客户端」会对每个 peer 直连一次标准 BT
        握手识别其真实客户端 —— 这是所有下载器拿到 peer 列表后的正常行为,tracker
        不感知,不产生账号风险。
      </p>
      <button
        class="flex flex-shrink-0 cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-background px-2.5 py-1.5 text-xs font-medium text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
        onclick={probeClients}
        disabled={probing || !instanceId || !uniquePeers.length}
      >
        <Radar size={13} class={probing ? 'animate-pulse' : ''} />
        {probing ? '探测中…' : '探测客户端'}
      </button>
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
              {#each uniquePeers as peer, i (peer.ip + ':' + peer.port + '-' + i)}
                {@const cell = clientCell(peer)}
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
                  <td class="px-3 py-1.5 whitespace-nowrap">
                    {#if cell.online === true}
                      <span
                        class="mr-1.5 inline-block h-1.5 w-1.5 rounded-full bg-emerald-500"
                        title="握手成功:对方端口真实可达(真实在线)"
                      ></span>
                    {:else if cell.online === false}
                      <span
                        class="mr-1.5 inline-block h-1.5 w-1.5 rounded-full bg-zinc-400"
                        title="未响应:对方离线或位于 NAT 后无法直连(不代表对方不在线做种)"
                      ></span>
                    {/if}
                    {#if isSelf(peer)}
                      <span class="text-muted-foreground" title="本机实例,不探测">本机实例</span>
                    {:else if cell.name}
                      <span title="peer_id: {cell.rawId}">{cell.name}</span>
                    {:else}
                      <span class="text-muted-foreground" title="点击「探测客户端」识别对方客户端">
                        {cell.online === false ? '未响应' : '—'}
                      </span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="mt-2 text-[10px] text-muted-foreground italic">
          共 {uniquePeers.length} 个 peer(重复行已合并)。IP 为对方与 tracker 通信的地址,可能与对方真实出口不同
          (NAT/代理)。<br />
          客户端识别来自直连握手获取的对方 peer_id(BT 标准编码):绿点 = 握手成功(对方端口真实可达);
          灰点 = 未响应(对方离线/NAT 后无法直连,不代表不在线做种)。探测使用本实例的 info_hash
          与伪装 peer_id(与 tracker 汇报指纹一致),握手后立即断开、不交换任何数据;tracker
          完全感知不到 peer 直连,不会带来任何账号风险。
        </p>
      {/if}
    </div>
  </BaseModal>
{/if}
