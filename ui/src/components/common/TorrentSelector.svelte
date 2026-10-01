<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import {
    FileText,
    FolderOpen,
    File,
    Key,
    Globe,
    Files,
    ChevronDown,
    ChevronRight,
    Upload,
    ExternalLink,
    Sprout,
    Download,
  } from '@lucide/svelte';
  import { getTrackerSiteUrl } from '$lib/trackerUtils.js';

  let {
    torrent,
    selectTorrent,
    formatBytes,
    completionPercent = 100,
    isRunning = false,
    onUpdate = () => {},
  } = $props();

  let showDetails = $state(false);
  let isDragging = $state(false);
  let fileInput;

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // 在系统默认浏览器中打开(桌面端用 shell 插件,网页端用 window.open)
  async function openInBrowser(url) {
    if (!url) return;
    if (isTauri) {
      try {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open(url);
      } catch (e) {
        console.error('打开网站失败:', e);
        window.open(url, '_blank', 'noopener');
      }
    } else {
      window.open(url, '_blank', 'noopener');
    }
  }

  function getAllTrackers(torrent) {
    if (!torrent) return [];
    const trackers = new Set();
    if (torrent.announce) trackers.add(torrent.announce);
    if (torrent.announce_list && Array.isArray(torrent.announce_list)) {
      torrent.announce_list.forEach(tier => {
        if (Array.isArray(tier)) {
          tier.forEach(url => trackers.add(url));
        }
      });
    }
    return Array.from(trackers);
  }

  async function handleFileSelect() {
    if (isTauri) {
      // Use Tauri file dialog
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Torrent', extensions: ['torrent'] }],
      });
      if (selected) {
        await selectTorrent(selected);
      }
    } else {
      // Use HTML5 file input
      fileInput.click();
    }
  }

  async function handleFileChange(event) {
    const file = event.target.files?.[0];
    if (file) {
      await selectTorrent(file);
    }
  }

  function handleDragOver(event) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = true;
  }

  function handleDragLeave(event) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = false;
  }

  async function handleDrop(event) {
    event.preventDefault();
    event.stopPropagation();
    isDragging = false;

    const files = event.dataTransfer?.files;
    if (files && files.length > 0) {
      const file = files[0];
      // Check if it's a .torrent file
      if (file.name.endsWith('.torrent') || file.type === 'application/x-bittorrent') {
        await selectTorrent(file);
      } else {
        alert('请拖入 .torrent 种子文件');
      }
    }
  }

  let trackers = $derived(getAllTrackers(torrent));
  let siteUrl = $derived(getTrackerSiteUrl(torrent?.announce || trackers[0] || ''));
  let siteHost = $derived(siteUrl ? siteUrl.replace(/^https:\/\//, '') : '');

  // 运行状态:做种(完成度 100%)/ 下载中(可设完成度),与配置页「状态模式」联动
  let isSeeding = $derived((completionPercent ?? 100) >= 100);

  function setStatusMode(mode) {
    if (isRunning) return;
    const next = mode === 'seeding' ? 100 : 0;
    if ((completionPercent ?? 0) === next) return;
    onUpdate({ completionPercent: next });
  }
</script>

<Card class="p-3">
  <h2 class="mb-3 text-primary text-lg font-semibold flex items-center gap-2">
    <FileText size={20} />种子文件</h2>

  <input
    type="file"
    accept=".torrent"
    bind:this={fileInput}
    onchange={handleFileChange}
    class="hidden"
  />

  {#if torrent}
    <!-- Torrent loaded state -->
    <div class="bg-muted/50 rounded-lg border border-border overflow-hidden">
      <!-- Main info row -->
      <div class="p-3 flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0"
        >
          <File size={20} class="text-primary" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="font-medium text-sm truncate" title={torrent.name}>
            {torrent.name}
          </div>
          <div class="text-xs text-muted-foreground flex items-center gap-2 mt-0.5">
            <span>{formatBytes(torrent.total_size)}</span>
            <span class="text-border">•</span>
            <span>
              {torrent.file_count || torrent.files?.length || 1} 个文件
            </span>
            <span class="text-border">•</span>
            <span>{trackers.length} 个 Tracker</span>
          </div>
        </div>
        <Button onclick={handleFileSelect} variant="outline" class="h-8 px-3 text-xs">
          {#snippet children()}
            <span class="flex items-center gap-1.5">
              <FolderOpen size={14} />更换</span>
          {/snippet}
        </Button>
      </div>

      <!-- Quick stats -->
      <div class="grid grid-cols-4 border-t border-border">
        <div class="p-2 text-center border-r border-border">
          <div class="text-xs text-muted-foreground mb-0.5">大小</div>
          <div class="text-sm font-medium">{formatBytes(torrent.total_size)}</div>
        </div>
        <div class="p-2 text-center border-r border-border">
          <div class="text-xs text-muted-foreground mb-0.5">块数</div>
          <div class="text-sm font-medium">{torrent.num_pieces?.toLocaleString() || 'N/A'}</div>
        </div>
        <div class="p-2 text-center border-r border-border">
          <div class="text-xs text-muted-foreground mb-0.5">块大小</div>
          <div class="text-sm font-medium">
            {torrent.piece_length ? formatBytes(torrent.piece_length) : 'N/A'}
          </div>
        </div>
        <div class="p-2 text-center">
          <div class="text-xs text-muted-foreground mb-0.5">文件</div>
          <div class="text-sm font-medium">
            {torrent.file_count || torrent.files?.length || 1}
          </div>
        </div>
      </div>

      <!-- Details toggle -->
      <button
        class="w-full p-2 flex items-center justify-center gap-2 text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors border-t border-border cursor-pointer bg-transparent"
        onclick={() => (showDetails = !showDetails)}
      >
        {#if showDetails}
          <ChevronDown size={14} />
        {:else}
          <ChevronRight size={14} />
        {/if}
        {showDetails ? '收起详情' : '显示详情'}
      </button>

      <!-- Expanded details -->
      {#if showDetails}
        <div class="border-t border-border p-3 flex flex-col gap-3 bg-background/50">
          <!-- Info Hash -->
          <div>
            <div class="text-xs text-muted-foreground mb-1.5 flex items-center gap-1.5">
              <Key size={12} />Info 哈希</div>
            <code
              class="bg-muted text-primary px-2 py-1.5 rounded text-xs break-all font-mono block"
            >
              {torrent.info_hash
                ? Array.from(torrent.info_hash)
                    .map(b => b.toString(16).padStart(2, '0'))
                    .join('')
                : 'N/A'}
            </code>
          </div>

          <!-- Trackers -->
          {#if trackers.length > 0}
            <div>
              <div class="text-xs text-muted-foreground mb-1.5 flex items-center gap-1.5">
                <Globe size={12} /> Tracker 列表 ({trackers.length})
              </div>
              <div class="flex flex-col gap-1 max-h-[100px] overflow-y-auto">
                {#each trackers as tracker, index (tracker)}
                  <div class="flex items-center gap-2 text-xs">
                    {#if index === 0}
                      <span
                        class="px-1.5 py-0.5 rounded text-[0.6rem] font-medium uppercase bg-primary text-primary-foreground flex-shrink-0"
                      >主 Tracker</span>
                    {:else}
                      <span class="text-muted-foreground w-12 flex-shrink-0 text-right"
                        >#{index + 1}</span
                      >
                    {/if}
                    <code class="text-stat-upload break-all font-mono flex-1 min-w-0">
                      {tracker}
                    </code>
                    {#if getTrackerSiteUrl(tracker)}
                      <button
                        class="inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-primary/10 text-primary hover:bg-primary/20 transition-colors cursor-pointer border-0 flex-shrink-0"
                        title={`打开 ${getTrackerSiteUrl(tracker)}`}
                        onclick={() => openInBrowser(getTrackerSiteUrl(tracker))}
                      >
                        <ExternalLink size={11} />打开
                      </button>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- File List -->
          {#if torrent.files && torrent.files.length > 0}
            <div>
              <div class="text-xs text-muted-foreground mb-1.5 flex items-center gap-1.5">
                <Files size={12} /> 文件列表 ({torrent.files.length})
              </div>
              {#if torrent.files.length <= 10}
                <div class="flex flex-col gap-1 max-h-[150px] overflow-y-auto">
                  {#each torrent.files as file (file.path)}
                    <div
                      class="flex items-center justify-between gap-2 p-1.5 bg-muted rounded text-xs"
                    >
                      <span class="font-mono truncate flex-1 min-w-0">
                        {file.path?.join('/') || '未知'}
                      </span>
                      <span class="text-muted-foreground flex-shrink-0">
                        {formatBytes(file.length)}
                      </span>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-xs text-muted-foreground italic">
                  共 {torrent.files.length} 个文件(过多,不予逐个显示)
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- 种子所属网站:醒目大横幅,整块可点击,在浏览器打开并登录 -->
    {#if siteUrl}
      <button
        class="mt-3 w-full rounded-xl border-2 border-primary/30 bg-primary/5 hover:bg-primary/10 hover:border-primary/60 transition-all p-4 flex items-center gap-3 text-left cursor-pointer group bg-transparent"
        title={`在浏览器打开 ${siteUrl}`}
        onclick={() => openInBrowser(siteUrl)}
      >
        <div
          class="w-11 h-11 rounded-lg bg-primary/15 flex items-center justify-center flex-shrink-0"
        >
          <Globe size={22} class="text-primary" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-xs text-muted-foreground mb-0.5">种子所属网站(点击打开登录)</div>
          <div class="text-lg font-bold text-primary truncate">{siteHost}</div>
        </div>
        <div
          class="flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-primary text-primary-foreground text-sm font-semibold flex-shrink-0 group-hover:bg-primary/90 group-hover:-translate-y-0.5 transition-all shadow-lg shadow-primary/25"
        >
          <ExternalLink size={14} />打开网站
        </div>
      </button>
    {/if}

    <!-- 运行状态切换:做种 / 下载中 -->
    <div class="mt-3 rounded-xl border-2 border-border bg-muted/30 p-3">
      <div class="flex items-center justify-between mb-2">
        <span class="text-xs font-medium text-muted-foreground">运行状态</span>
        {#if isRunning}
          <span class="text-[10px] text-orange-500">停止实例后可切换</span>
        {/if}
      </div>
      <div class="grid grid-cols-2 gap-2">
        <button
          class="p-2.5 rounded-lg border-2 flex flex-col items-center gap-1 transition-all cursor-pointer {isSeeding
            ? 'border-stat-upload bg-stat-upload/10 text-stat-upload'
            : 'border-border bg-transparent text-muted-foreground hover:border-primary/40 hover:text-foreground'} {isRunning
            ? 'opacity-60 cursor-not-allowed'
            : ''}"
          disabled={isRunning}
          onclick={() => setStatusMode('seeding')}
          title="种子已下载完成,纯做种(完成度 100%)"
        >
          <Sprout size={20} />
          <span class="text-sm font-semibold">做种</span>
          <span class="text-[10px] {isSeeding ? 'text-stat-upload' : 'text-muted-foreground'}">
            {isSeeding ? '当前模式' : '完整做种 100%'}
          </span>
        </button>
        <button
          class="p-2.5 rounded-lg border-2 flex flex-col items-center gap-1 transition-all cursor-pointer {!isSeeding
            ? 'border-stat-leecher bg-stat-leecher/10 text-stat-leecher'
            : 'border-border bg-transparent text-muted-foreground hover:border-primary/40 hover:text-foreground'} {isRunning
            ? 'opacity-60 cursor-not-allowed'
            : ''}"
          disabled={isRunning}
          onclick={() => setStatusMode('downloading')}
          title="模拟下载,可自定义完成度"
        >
          <Download size={20} />
          <span class="text-sm font-semibold">下载中</span>
          <span class="text-[10px] {!isSeeding
            ? 'text-stat-leecher'
            : 'text-muted-foreground'}">
            {!isSeeding ? '当前模式' : '可设完成度'}
          </span>
        </button>
      </div>
    </div>
  {:else}
    <!-- Empty state with drag and drop -->
    <button
      onclick={handleFileSelect}
      ondragover={handleDragOver}
      ondragleave={handleDragLeave}
      ondrop={handleDrop}
      class="w-full p-6 border-2 border-dashed rounded-lg flex flex-col items-center gap-3 cursor-pointer transition-all group
        {isDragging
        ? 'border-primary bg-primary/10'
        : 'border-border bg-muted/30 hover:bg-muted/50 hover:border-primary/50'}"
    >
      <div
        class="w-12 h-12 rounded-full flex items-center justify-center transition-colors
        {isDragging ? 'bg-primary/20' : 'bg-muted group-hover:bg-primary/10'}"
      >
        <Upload
          size={24}
          class="transition-colors {isDragging
            ? 'text-primary'
            : 'text-muted-foreground group-hover:text-primary'}"
        />
      </div>
      <div class="text-center">
        <div class="font-medium text-sm mb-1">
          {isDragging ? '拖放种子文件到此处' : '选择种子文件'}
        </div>
        <div class="text-xs text-muted-foreground">
          {isDragging ? '松开以加载' : '点击选择或拖放文件'}
        </div>
      </div>
    </button>
  {/if}
</Card>
