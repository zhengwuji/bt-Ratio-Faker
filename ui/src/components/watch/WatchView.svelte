<script>
  import { onDestroy, onMount } from 'svelte';
  import { api, getRunMode } from '$lib/api.js';
  import { instanceActions } from '$lib/instanceStore.js';
  import {
    DEFAULT_PRESET_CHANGED_EVENT,
    getDefaultPreset,
    refreshDefaultPreset,
  } from '$lib/defaultPreset.js';
  import { watchFocusQuery } from '$lib/watchViewState.js';
  import {
    buildWatchTree,
    collectFolderIds,
    filterWatchFiles,
    flattenTree,
    folderId,
    isWithinPath,
    normalizePath,
  } from '$lib/watchTree.js';
  import { cn } from '$lib/utils.js';
  import Button from '$lib/components/ui/button.svelte';
  import ConfirmDialog from '../common/ConfirmDialog.svelte';
  import {
    ChevronRight,
    Folder,
    FolderOpen,
    File,
    CheckCircle2,
    Clock3,
    AlertTriangle,
    RefreshCw,
    RotateCw,
    Trash2,
    Search,
    HardDrive,
    FolderUp,
    X,
  } from '@lucide/svelte';

  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  let watchStatus = $state(null);
  let watchConfig = $state({ max_depth: 1, auto_start: false, watch_dir: '' });
  let watchDefaultPresetName = $state('Rustatio defaults');
  let watchFiles = $state([]);
  let isLoading = $state(false);
  let isReloading = $state(false);
  let reloadingFile = $state(null);
  let deletingFile = $state(null);
  let deletingFolder = $state(null);
  let pickingFolder = $state(false);
  let saveConfigBusy = $state(false);
  let error = $state('');
  let searchQuery = $state('');
  let statusFilter = $state('all');
  let expandedIds = $state(new Set());
  let isEmptyExpanded = $state(true);
  let lastWatchKey = $state('');
  let isWatchMode = $derived(getRunMode() === 'server' || getRunMode() === 'desktop');
  let pendingFocusQuery = $state('');
  let focusAppliedAt = 0;
  let actionConfirmVisible = $state(false);
  let actionConfirmTitle = $state('Confirm Action');
  let actionConfirmMessage = $state('');
  let actionConfirmOkLabel = $state('Confirm');
  let actionConfirmCancelLabel = $state('Cancel');
  let actionConfirmKind = $state('info');
  let actionConfirmResolve = null;
  let isLoadingGuard = false;
  let pollInterval = null;
  let removeDefaultPresetListener = null;
  let selectedPaths = $state(new Set());
  let deletingSelected = $state(false);
  let reloadingSelected = $state(false);
  let lastSelectedPath = $state('');

  const statusTabs = [
    { key: 'all', label: 'All' },
    { key: 'pending', label: 'Pending' },
    { key: 'loaded', label: 'Loaded' },
    { key: 'invalid', label: 'Invalid' },
  ];

  function getStatusBadge(status) {
    switch (status) {
      case 'loaded':
        return 'bg-stat-upload/20 text-stat-upload border-stat-upload/30';
      case 'pending':
        return 'bg-stat-ratio/20 text-stat-ratio border-stat-ratio/30';
      case 'invalid':
        return 'bg-stat-leecher/20 text-stat-leecher border-stat-leecher/30';
      default:
        return 'bg-muted/70 text-muted-foreground border-border';
    }
  }

  function getStatusIcon(status) {
    if (status === 'loaded') return CheckCircle2;
    if (status === 'pending') return Clock3;
    if (status === 'invalid') return AlertTriangle;
    return File;
  }

  function buildWatchKey(files = []) {
    return files
      .map(file => `${normalizePath(file?.filename)}:${file?.status || ''}`)
      .sort()
      .join('|');
  }

  function trimPath(path) {
    return String(path || '')
      .replace(/\\/g, '/')
      .replace(/\/+/g, '/')
      .trim();
  }

  function folderFiles(folderPath) {
    return watchFiles.filter(file => {
      const name = normalizePath(file?.filename);
      return isWithinPath(name, folderPath);
    });
  }

  function askActionConfirm({
    title,
    message,
    okLabel = 'Confirm',
    cancelLabel = 'Cancel',
    kind = 'info',
  }) {
    actionConfirmTitle = title;
    actionConfirmMessage = message;
    actionConfirmOkLabel = okLabel;
    actionConfirmCancelLabel = cancelLabel;
    actionConfirmKind = kind;
    actionConfirmVisible = true;
    return new Promise(resolve => {
      actionConfirmResolve = resolve;
    });
  }

  function resolveActionConfirm(result) {
    actionConfirmVisible = false;
    const resolve = actionConfirmResolve;
    actionConfirmResolve = null;
    if (resolve) {
      resolve(result);
    }
  }

  function indeterminate(node, value) {
    node.indeterminate = Boolean(value);
    return {
      update(next) {
        node.indeterminate = Boolean(next);
      },
    };
  }

  async function confirmFolderDelete(folderPath, count) {
    return askActionConfirm({
      title: '删除文件夹中的文件',
      message: `删除"${folderPath || '根目录'}"中的 ${count} 个种子文件?`,
      okLabel: '全部删除',
      cancelLabel: '取消',
      kind: 'warning',
    });
  }

  async function confirmSelectedDelete(count) {
    return askActionConfirm({
      title: '删除所选文件',
      message: `删除所选的 ${count} 个种子文件?`,
      okLabel: '删除所选',
      cancelLabel: '取消',
      kind: 'warning',
    });
  }

  async function loadWatchData() {
    if (!isWatchMode) return;
    if (isLoadingGuard) return;

    isLoadingGuard = true;
    isLoading = true;
    error = '';

    try {
      const timeout = new Promise((_, reject) => {
        setTimeout(() => reject(new Error('监视文件夹请求超时')), 10000);
      });

      const [status, files, config] = await Promise.race([
        Promise.all([api.getWatchStatus(), api.listWatchFiles(), api.getWatchConfig()]),
        timeout,
      ]);

      watchStatus = status;
      watchFiles = files || [];
      watchConfig = {
        max_depth: Number(config?.max_depth ?? 1),
        auto_start: Boolean(config?.auto_start),
        watch_dir: String(config?.watch_dir || status?.watch_dir || ''),
      };

      await refreshDefaultPreset();
      watchDefaultPresetName = getDefaultPreset()?.name || 'Rustatio defaults';

      const nextKey = buildWatchKey(watchFiles);
      const needsReset = nextKey !== lastWatchKey;
      lastWatchKey = nextKey;

      if (needsReset && isEmptyExpanded) {
        expandedIds = collectFolderIds(buildWatchTree(watchFiles));
        isEmptyExpanded = expandedIds.size === 0;
      }
    } catch (e) {
      console.error('Failed to load watch folder data:', e);
      error = e.message;
    } finally {
      isLoading = false;
      isLoadingGuard = false;
    }
  }

  async function saveWatchConfig(partial) {
    const merged = {
      max_depth: Number(partial?.max_depth ?? watchConfig.max_depth ?? 1),
      auto_start: Boolean(partial?.auto_start ?? watchConfig.auto_start),
      watch_dir: trimPath(partial?.watch_dir ?? watchConfig.watch_dir ?? ''),
    };

    if (!Number.isFinite(merged.max_depth) || merged.max_depth < 0) {
      error = '最大深度必须不小于 0';
      return false;
    }

    saveConfigBusy = true;
    error = '';
    try {
      await api.setWatchConfig({
        max_depth: Math.floor(merged.max_depth),
        auto_start: merged.auto_start,
        watch_dir: merged.watch_dir,
      });
      watchConfig = { ...merged, max_depth: Math.floor(merged.max_depth) };
      await loadWatchData();
      return true;
    } catch (e) {
      console.error('Failed to save watch config:', e);
      error = e.message || '保存监视配置失败';
      return false;
    } finally {
      saveConfigBusy = false;
    }
  }

  async function handleWatchDirCommit() {
    await saveWatchConfig({ watch_dir: watchConfig.watch_dir });
  }

  async function handleMaxDepthCommit() {
    await saveWatchConfig({ max_depth: watchConfig.max_depth });
  }

  async function handleAutoStartChange(event) {
    if (saveConfigBusy) {
      if (event?.target) {
        event.target.checked = Boolean(watchConfig.auto_start);
      }
      return;
    }

    const previous = Boolean(watchConfig.auto_start);
    const next = Boolean(event?.target?.checked);

    if (next === previous) return;

    let summaries;
    try {
      summaries = await api.listSummaries();
    } catch (e) {
      console.error('Failed to list summaries for auto-start prompt:', e);
      error = e.message || '加载监视文件夹实例失败';
      watchConfig.auto_start = previous;
      if (event?.target) {
        event.target.checked = previous;
      }
      return;
    }

    const watchInstances = (summaries || []).filter(s => s.source === 'watch_folder');
    const targets = watchInstances.map(s => String(s.id)).filter(id => id.length > 0);

    if (next) {
      if (targets.length > 0) {
        const ask = await askActionConfirm({
          title: '立即开始监视种子',
          message: `已开启自动开始。现在开始 ${targets.length} 个监视文件夹中的种子?`,
          okLabel: '立即开始',
          cancelLabel: '稍后',
          kind: 'info',
        });

        if (ask) {
          await api.gridStart(targets);
          await instanceActions.reconcileWithBackend();
          await loadWatchData();
        }
      }

      watchConfig.auto_start = next;
      const saved = await saveWatchConfig({ auto_start: next });
      if (!saved) {
        watchConfig.auto_start = previous;
      }
      return;
    }

    const runningTargets = watchInstances
      .filter(s =>
        ['running', 'starting', 'paused', 'idle'].includes(String(s.state || '').toLowerCase())
      )
      .map(s => String(s.id))
      .filter(id => id.length > 0);

    if (runningTargets.length === 0) {
      watchConfig.auto_start = next;
      const saved = await saveWatchConfig({ auto_start: next });
      if (!saved) {
        watchConfig.auto_start = previous;
        if (event?.target) {
          event.target.checked = previous;
        }
      }
      return;
    }

    const askStop = await askActionConfirm({
      title: '立即停止监视种子',
      message: `已关闭自动开始。现在停止 ${runningTargets.length} 个监视文件夹中的种子?`,
      okLabel: '立即停止',
      cancelLabel: '继续运行',
      kind: 'warning',
    });

    if (askStop) {
      await api.gridStop(runningTargets);
      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    }

    watchConfig.auto_start = next;
    const saved = await saveWatchConfig({ auto_start: next });
    if (!saved) {
      watchConfig.auto_start = previous;
      if (event?.target) {
        event.target.checked = previous;
      }
    }
  }

  async function pickWatchDirectory() {
    if (!isTauri) return;
    pickingFolder = true;
    error = '';
    try {
      const dialog = await import('@tauri-apps/plugin-dialog');
      const selected = await dialog.open({
        directory: true,
        multiple: false,
        defaultPath: watchConfig.watch_dir || undefined,
      });
      if (!selected) return;

      const value = Array.isArray(selected) ? selected[0] : selected;
      if (!value) return;

      await saveWatchConfig({ watch_dir: String(value) });
    } catch (e) {
      console.error('Failed to pick watch directory:', e);
      error = e.message || '选择监视目录失败';
    } finally {
      pickingFolder = false;
    }
  }

  async function handleDeleteFile(filename) {
    deletingFile = filename;
    try {
      await api.deleteWatchFile(filename);
      const next = new Set(selectedPaths);
      next.delete(String(filename));
      selectedPaths = next;
      if (lastSelectedPath === String(filename)) {
        lastSelectedPath = '';
      }
      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    } catch (e) {
      console.error('Failed to delete file:', e);
      error = e.message;
    } finally {
      deletingFile = null;
    }
  }

  async function handleDeleteSelected() {
    const targets = Array.from(selectedPaths);
    if (targets.length === 0) return;

    const confirmed = await confirmSelectedDelete(targets.length);
    if (!confirmed) return;

    deletingSelected = true;
    error = '';
    try {
      const jobs = targets.map(path => api.deleteWatchFile(path));
      await Promise.all(jobs);
      selectedPaths = new Set();
      lastSelectedPath = '';
      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    } catch (e) {
      console.error('Failed to delete selected files:', e);
      error = e.message || '删除所选文件失败';
    } finally {
      deletingSelected = false;
    }
  }

  async function handleReloadSelected() {
    const targets = Array.from(selectedPaths);
    if (targets.length === 0) return;

    reloadingSelected = true;
    error = '';
    try {
      const jobs = targets.map(path => api.reloadWatchFile(path));
      const results = await Promise.allSettled(jobs);
      const failed = results.filter(result => result.status === 'rejected');

      if (failed.length > 0) {
        const reason = failed[0]?.reason;
        const detail = reason?.message || String(reason || 'Unknown error');
        if (failed.length === targets.length) {
          error = `重新加载所选文件失败:${detail}`;
        } else {
          const okCount = targets.length - failed.length;
          error = `已重新加载 ${okCount}/${targets.length} 个所选文件,${failed.length} 个失败:${detail}`;
        }
      }

      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    } catch (e) {
      console.error('Failed to reload selected files:', e);
      error = e.message || '重新加载所选文件失败';
    } finally {
      reloadingSelected = false;
    }
  }

  function toggleFileSelected(path, checked, shiftKey = false) {
    const key = String(path || '');
    if (!key) return;

    const shouldSelect = checked ?? !selectedPaths.has(key);

    if (shiftKey && lastSelectedPath && lastSelectedPath !== key) {
      const start = visibleFilePaths.indexOf(lastSelectedPath);
      const end = visibleFilePaths.indexOf(key);
      if (start !== -1 && end !== -1) {
        const from = Math.min(start, end);
        const to = Math.max(start, end);
        const next = new Set(selectedPaths);
        for (let i = from; i <= to; i += 1) {
          const rangePath = visibleFilePaths[i];
          if (shouldSelect) {
            next.add(rangePath);
          } else {
            next.delete(rangePath);
          }
        }
        selectedPaths = next;
        lastSelectedPath = key;
        return;
      }
    }

    const next = new Set(selectedPaths);
    if (shouldSelect) {
      next.add(key);
    } else {
      next.delete(key);
    }
    selectedPaths = next;
    lastSelectedPath = key;
  }

  function toggleVisibleSelection(checked) {
    const shouldSelect = Boolean(checked);
    const next = new Set(selectedPaths);
    for (const path of visibleFilePaths) {
      if (shouldSelect) {
        next.add(path);
      } else {
        next.delete(path);
      }
    }
    selectedPaths = next;
  }

  function clearSelection() {
    selectedPaths = new Set();
    lastSelectedPath = '';
  }

  function toggleFolderSelection(folderPath, checked) {
    const info = folderSelectionState.get(folderPath);
    if (!info || info.total === 0) return;

    const next = new Set(selectedPaths);
    for (const path of info.paths) {
      if (checked) {
        next.add(path);
      } else {
        next.delete(path);
      }
    }
    selectedPaths = next;
  }

  function isInteractiveTarget(target) {
    if (!(target instanceof Element)) return false;
    return Boolean(target.closest('button, input, select, textarea, a, label'));
  }

  function handleFileRowClick(event, path) {
    if (isInteractiveTarget(event.target)) return;
    toggleFileSelected(path, undefined, Boolean(event.shiftKey));
  }

  function handleFileRowKeydown(event, path) {
    if (event.target !== event.currentTarget) return;
    if (event.key !== 'Enter' && event.key !== ' ') return;
    event.preventDefault();
    toggleFileSelected(path, undefined, Boolean(event.shiftKey));
  }

  async function handleDeleteFolder(folderPath) {
    const currentFiles = folderFiles(folderPath);
    if (currentFiles.length === 0) return;

    const confirmed = await confirmFolderDelete(folderPath, currentFiles.length);
    if (!confirmed) return;

    const files = folderFiles(folderPath);
    if (files.length === 0) return;

    deletingFolder = folderPath || '/';
    error = '';
    try {
      const jobs = files.map(file => api.deleteWatchFile(file.filename));
      await Promise.all(jobs);
      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    } catch (e) {
      console.error('Failed to delete folder files:', e);
      error = e.message || '删除文件夹文件失败';
    } finally {
      deletingFolder = null;
    }
  }

  async function handleReloadFile(filename) {
    reloadingFile = filename;
    try {
      await api.reloadWatchFile(filename);
      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    } catch (e) {
      console.error('Failed to reload file:', e);
      error = e.message;
    } finally {
      reloadingFile = null;
    }
  }

  async function handleReloadAll() {
    isReloading = true;
    error = '';
    try {
      await api.reloadAllWatchFiles();
      await instanceActions.reconcileWithBackend();
      await loadWatchData();
    } catch (e) {
      console.error('Failed to reload all files:', e);
      error = e.message;
    } finally {
      isReloading = false;
    }
  }

  function toggleFolder(path) {
    const id = folderId(path);
    const next = new Set(expandedIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedIds = next;
    isEmptyExpanded = expandedIds.size === 0;
  }

  function expandAll() {
    expandedIds = collectFolderIds(buildWatchTree(filteredFiles));
    isEmptyExpanded = expandedIds.size === 0;
  }

  function collapseAll() {
    expandedIds = new Set([folderId('')]);
    isEmptyExpanded = expandedIds.size === 0;
  }

  function formatSize(bytes) {
    if (!bytes) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let size = Number(bytes);
    let idx = 0;
    while (size >= 1024 && idx < units.length - 1) {
      size /= 1024;
      idx += 1;
    }
    return `${size.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
  }

  let filteredFiles = $derived.by(() => filterWatchFiles(watchFiles, searchQuery, statusFilter));
  let watchTree = $derived.by(() => buildWatchTree(filteredFiles));
  let watchRows = $derived.by(() => flattenTree(watchTree, expandedIds));
  let displayRows = $derived.by(() =>
    watchRows.filter(row => !(row.type === 'folder' && row.path === ''))
  );
  let folderCount = $derived.by(() => Math.max(0, collectFolderIds(watchTree).size - 1));
  let fileCount = $derived.by(() => filteredFiles.length);
  let loadedCount = $derived.by(
    () => watchFiles.filter(file => String(file?.status || '').toLowerCase() === 'loaded').length
  );
  let pendingCount = $derived.by(
    () => watchFiles.filter(file => String(file?.status || '').toLowerCase() === 'pending').length
  );
  let invalidCount = $derived.by(
    () => watchFiles.filter(file => String(file?.status || '').toLowerCase() === 'invalid').length
  );
  let totalOnDiskCount = $derived.by(() => watchFiles.length);
  let visibleFilePaths = $derived.by(() =>
    displayRows.filter(row => row.type === 'file').map(row => String(row.path))
  );
  let selectedCount = $derived.by(() => selectedPaths.size);
  let allVisibleSelected = $derived.by(
    () => visibleFilePaths.length > 0 && visibleFilePaths.every(path => selectedPaths.has(path))
  );
  let someVisibleSelected = $derived.by(
    () => visibleFilePaths.some(path => selectedPaths.has(path)) && !allVisibleSelected
  );
  let folderSelectionState = $derived.by(() => {
    const state = new Map();

    for (const row of displayRows) {
      if (row.type !== 'folder') continue;

      const paths = visibleFilePaths.filter(path => isWithinPath(path, row.path));
      let selected = 0;
      for (const path of paths) {
        if (selectedPaths.has(path)) selected += 1;
      }

      const total = paths.length;
      const all = total > 0 && selected === total;
      const some = selected > 0 && !all;
      state.set(row.path, { paths, total, all, some });
    }

    return state;
  });
  let filteredMap = $derived.by(() => {
    const map = new Map();
    for (const file of filteredFiles) {
      map.set(file.filename, file);
    }
    return map;
  });

  function isSameSet(a, b) {
    if (a.size !== b.size) return false;
    for (const value of a) {
      if (!b.has(value)) return false;
    }
    return true;
  }

  $effect(() => {
    const safeExpanded = new Set();
    for (const id of expandedIds) {
      if (id === folderId('')) {
        safeExpanded.add(id);
        continue;
      }
      const path = id.replace('folder:', '');
      const exists = filteredFiles.some(file => isWithinPath(file.filename, path));
      if (exists) safeExpanded.add(id);
    }
    if (safeExpanded.size === 0) {
      safeExpanded.add(folderId(''));
    }
    if (!isSameSet(safeExpanded, expandedIds)) {
      expandedIds = safeExpanded;
      isEmptyExpanded = expandedIds.size === 0;
    }
  });

  $effect(() => {
    const available = new Set(visibleFilePaths);
    const next = new Set();
    for (const path of selectedPaths) {
      if (available.has(path)) {
        next.add(path);
      }
    }
    if (!isSameSet(next, selectedPaths)) {
      selectedPaths = next;
    }

    if (lastSelectedPath && !available.has(lastSelectedPath)) {
      lastSelectedPath = '';
    }
  });

  onMount(() => {
    const syncDefaultPresetName = async () => {
      await refreshDefaultPreset();
      watchDefaultPresetName = getDefaultPreset()?.name || 'Rustatio defaults';
    };

    syncDefaultPresetName();
    window.addEventListener(DEFAULT_PRESET_CHANGED_EVENT, syncDefaultPresetName);
    removeDefaultPresetListener = () => {
      window.removeEventListener(DEFAULT_PRESET_CHANGED_EVENT, syncDefaultPresetName);
    };

    if (!isWatchMode) return;
    loadWatchData();
    pollInterval = setInterval(loadWatchData, 15000);
  });

  $effect(() => {
    const query = $watchFocusQuery;
    if (!query) return;
    pendingFocusQuery = String(query);
    watchFocusQuery.set('');
  });

  $effect(() => {
    if (!pendingFocusQuery) return;
    if (isLoading) return;

    const now = Date.now();
    // Prevent rapid duplicate applications during re-renders
    if (now - focusAppliedAt < 100) return;
    focusAppliedAt = now;

    searchQuery = pendingFocusQuery;
    statusFilter = 'all';
    expandAll();
    pendingFocusQuery = '';
  });

  onDestroy(() => {
    if (actionConfirmResolve) {
      resolveActionConfirm(false);
    }

    if (removeDefaultPresetListener) {
      removeDefaultPresetListener();
      removeDefaultPresetListener = null;
    }

    isLoadingGuard = false;
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
  });
</script>

<div class="flex flex-col gap-2 h-full w-full max-w-none">
  <div class="rounded-xl border border-border bg-card">
    <div
      class="px-3 py-2.5 md:px-3.5 md:py-3 border-b border-border flex flex-wrap items-center gap-2 justify-between"
    >
      <div class="flex items-center gap-2 min-w-0">
        <HardDrive size={16} class="text-primary" />
        <h2 class="text-base font-semibold truncate">监视浏览器</h2>
      </div>
      <div class="flex items-center gap-2 flex-wrap justify-end">
        <div class="hidden lg:flex items-center gap-1.5">
          <span
            class="inline-flex items-center gap-1 rounded-md border border-stat-upload/30 bg-stat-upload/10 px-2 py-0.5 text-[10px] font-medium text-stat-upload"
            title="已加载文件"
          >已加载<span class="font-semibold">{loadedCount}</span>
          </span>
          <span
            class="inline-flex items-center gap-1 rounded-md border border-stat-ratio/30 bg-stat-ratio/10 px-2 py-0.5 text-[10px] font-medium text-stat-ratio"
            title="待处理文件"
          >待处理<span class="font-semibold">{pendingCount}</span>
          </span>
          <span
            class="inline-flex items-center gap-1 rounded-md border border-stat-leecher/30 bg-stat-leecher/10 px-2 py-0.5 text-[10px] font-medium text-stat-leecher"
            title="无效文件"
          >无效<span class="font-semibold">{invalidCount}</span>
          </span>
          <span
            class="inline-flex items-center gap-1 rounded-md border border-border bg-muted/30 px-2 py-0.5 text-[10px] font-medium text-muted-foreground"
            title="磁盘上文件总数"
          >总计<span class="font-semibold text-foreground">{totalOnDiskCount}</span>
          </span>
        </div>
        <Button
          onclick={loadWatchData}
          disabled={isLoading ||
            isReloading ||
            reloadingSelected ||
            deletingSelected ||
            Boolean(reloadingFile) ||
            Boolean(deletingFile) ||
            Boolean(deletingFolder) ||
            saveConfigBusy}
          size="sm"
          variant="outline"
          class="gap-1.5"
        >
          {#snippet children()}
            <RefreshCw size={12} class={cn(isLoading && 'animate-spin')} />
            {isLoading ? 'Loading' : 'Refresh'}
          {/snippet}
        </Button>
        <Button
          onclick={handleReloadAll}
          disabled={isLoading ||
            isReloading ||
            reloadingSelected ||
            deletingSelected ||
            Boolean(reloadingFile) ||
            Boolean(deletingFile) ||
            Boolean(deletingFolder) ||
            saveConfigBusy}
          size="sm"
          variant="outline"
          class="gap-1.5"
        >
          {#snippet children()}
            <RotateCw size={12} class={cn(isReloading && 'animate-spin')} />
            {isReloading ? 'Reloading' : 'Reload All'}
          {/snippet}
        </Button>
        <Button
          onclick={handleReloadSelected}
          disabled={selectedCount === 0 ||
            reloadingSelected ||
            deletingSelected ||
            isLoading ||
            isReloading ||
            Boolean(reloadingFile) ||
            Boolean(deletingFile) ||
            Boolean(deletingFolder) ||
            saveConfigBusy}
          size="sm"
          variant="outline"
          class="gap-1.5"
          title="重新加载选中文件"
        >
          {#snippet children()}
            {#if reloadingSelected}
              <RotateCw size={12} class="animate-spin" />
              Reloading ({selectedCount})
            {:else}
              <RotateCw size={12} />
              Reload Selected ({selectedCount})
            {/if}
          {/snippet}
        </Button>
        <Button
          onclick={handleDeleteSelected}
          disabled={selectedCount === 0 ||
            deletingSelected ||
            reloadingSelected ||
            isLoading ||
            isReloading ||
            Boolean(reloadingFile) ||
            Boolean(deletingFile) ||
            Boolean(deletingFolder) ||
            saveConfigBusy}
          size="sm"
          variant="outline"
          class="gap-1.5 text-stat-leecher border-stat-leecher/40 hover:bg-stat-leecher/10"
          title="删除选中文件"
        >
          {#snippet children()}
            {#if deletingSelected}
              <RotateCw size={12} class="animate-spin" />
              Deleting ({selectedCount})
            {:else}
              <Trash2 size={12} />
              Delete Selected ({selectedCount})
            {/if}
          {/snippet}
        </Button>
        <Button
          onclick={clearSelection}
          disabled={selectedCount === 0 || deletingSelected || reloadingSelected}
          size="sm"
          variant="outline"
          class="gap-1"
          title="清除选中文件"
        >
          {#snippet children()}
            <X size={12} />
            Clear
          {/snippet}
        </Button>
        <div class="text-[10px] text-muted-foreground hidden xl:block">仅可见选择</div>
      </div>
    </div>

    {#if !isWatchMode}
      <div class="p-4 text-sm text-muted-foreground">监视文件夹在服务器与桌面模式下可用。</div>
    {:else}
      <div
        class="px-2.5 pb-2.5 pt-2.5 md:px-3 md:pb-3 md:pt-3 grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)] gap-3 w-full"
      >
        <section class="space-y-2">
          <div class="rounded-lg border border-border px-2.5 py-2 space-y-1.5 bg-muted/20">
            <div class="text-xs uppercase tracking-wide text-muted-foreground">监视目录</div>
            <div class="flex items-start gap-2">
              <input
                bind:value={watchConfig.watch_dir}
                onchange={handleWatchDirCommit}
                onkeydown={event => event.key === 'Enter' && event.currentTarget.blur()}
                class="flex-1 px-2.5 py-2 text-sm rounded-md border border-border bg-background"
                placeholder="监视目录路径"
                disabled={saveConfigBusy}
              />
              {#if isTauri}
                <Button
                  onclick={pickWatchDirectory}
                  size="icon"
                  variant="outline"
                  disabled={pickingFolder || saveConfigBusy}
                  title="选择文件夹"
                >
                  {#snippet children()}
                    <FolderUp size={14} class={cn(pickingFolder && 'animate-pulse')} />
                  {/snippet}
                </Button>
              {/if}
            </div>
            <div class="flex items-center gap-2">
              <label class="text-[11px] text-muted-foreground" for="watchDepth">最大深度</label>
              <input
                id="watchDepth"
                type="number"
                min="0"
                step="1"
                class="w-18 px-2 py-1 text-xs rounded-md border border-border bg-background"
                bind:value={watchConfig.max_depth}
                onchange={handleMaxDepthCommit}
                disabled={saveConfigBusy}
              />
            </div>
            <div class="text-[10px] text-muted-foreground/80 leading-tight">
              Use <strong>0</strong>表示不限深度。</div>
            <label
              class="flex items-start gap-2 text-[11px] leading-tight text-muted-foreground cursor-pointer"
            >
              <input
                type="checkbox"
                class="mt-0.5"
                checked={watchConfig.auto_start}
                onchange={handleAutoStartChange}
                disabled={saveConfigBusy}
              />
              <span>应用启动及种子加载时自动开始<span class="block text-[11px] text-muted-foreground/80">
                  Rustatio 启动时,监视文件夹中的种子将自动开始。
                </span>
              </span>
            </label>
            {#if watchStatus}
              <div class="text-[11px] text-muted-foreground pt-0.5 leading-tight">当前路径:<span class="text-foreground">{watchStatus.watch_dir}</span>
              </div>
            {/if}
            <div class="text-[11px] text-muted-foreground leading-tight">已选预设:<span class="text-foreground">{watchDefaultPresetName}</span>
            </div>
          </div>

          <div class="rounded-lg border border-border px-2.5 py-2 space-y-1.5 bg-muted/20">
            <div
              class="flex items-center gap-2 text-xs uppercase tracking-wide text-muted-foreground"
            >
              <Search size={12} />筛选</div>
            <input
              type="text"
              class="w-full px-2.5 py-2 text-sm rounded-md border border-border bg-background"
              placeholder="名称或路径"
              bind:value={searchQuery}
            />
            <div class="flex flex-wrap gap-1">
              {#each statusTabs as tab (tab.key)}
                <button
                  onclick={() => (statusFilter = tab.key)}
                  class={cn(
                    'px-2 py-0.5 rounded-md text-[11px] border',
                    statusFilter === tab.key
                      ? 'bg-primary/15 text-primary border-primary/40'
                      : 'bg-background text-muted-foreground border-border hover:text-foreground'
                  )}
                >
                  {tab.label}
                </button>
              {/each}
            </div>
          </div>

          <div class="rounded-lg border border-border px-2.5 py-2 bg-muted/20">
            <div class="text-xs uppercase tracking-wide text-muted-foreground mb-2">概览</div>
            <div class="grid grid-cols-2 gap-x-2 gap-y-1 text-[11px] leading-tight">
              <div
                class="flex items-center justify-between rounded border border-border/60 bg-background/40 px-2 py-1"
              >
                <span>文件夹</span><span class="font-semibold">{folderCount}</span>
              </div>
              <div
                class="flex items-center justify-between rounded border border-border/60 bg-background/40 px-2 py-1"
              >
                <span>文件</span><span class="font-semibold">{fileCount}</span>
              </div>
              <div
                class="flex items-center justify-between rounded border border-stat-upload/40 bg-stat-upload/10 px-2 py-1"
              >
                <span>已加载</span><span class="font-semibold text-stat-upload">{loadedCount}</span>
              </div>
              <div
                class="flex items-center justify-between rounded border border-border/60 bg-background/40 px-2 py-1"
              >
                <span>磁盘总计</span><span class="font-semibold">{totalOnDiskCount}</span>
              </div>
            </div>
            <div class="grid grid-cols-2 gap-1.5 mt-2">
              <Button onclick={expandAll} size="sm" variant="outline" class="h-8">展开</Button>
              <Button onclick={collapseAll} size="sm" variant="outline" class="h-8">折叠</Button
              >
            </div>
          </div>
        </section>

        <section class="rounded-lg border border-border overflow-hidden bg-background/60 min-w-0">
          <div
            class="sticky top-0 z-10 grid grid-cols-[minmax(0,1fr)_96px_78px_84px] gap-2 px-2.5 py-1.5 text-[10px] uppercase tracking-wide text-muted-foreground bg-muted/60 border-b border-border backdrop-blur-sm"
          >
            <div class="flex items-center gap-2">
              <input
                use:indeterminate={someVisibleSelected}
                type="checkbox"
                checked={allVisibleSelected}
                disabled={visibleFilePaths.length === 0 || deletingSelected || reloadingSelected}
                onchange={event => toggleVisibleSelection(event.currentTarget.checked)}
                class="h-3.5 w-3.5 rounded-sm border border-primary/50 bg-background accent-primary disabled:opacity-50"
                title="选择可见文件"
                aria-label="选择可见文件"
              />
              <span>选择可见</span>
            </div>
            <div>状态</div>
            <div>大小</div>
            <div>操作</div>
          </div>

          <div class="max-h-[74vh] overflow-y-auto">
            {#if error}
              <div
                class="m-3 px-3 py-2 rounded-md border border-stat-leecher/30 bg-stat-leecher/10 text-sm text-stat-leecher"
              >
                {error}
              </div>
            {/if}

            {#if filteredFiles.length === 0 && !isLoading}
              <div class="py-10 text-center text-sm text-muted-foreground">没有文件符合当前筛选条件。</div>
            {:else}
              {#each displayRows as row (row.id)}
                {#if row.type === 'folder'}
                  {@const folderPath = row.path}
                  {@const count = row.count || 0}
                  {@const busy = deletingFolder === (folderPath || '/')}
                  {@const select = folderSelectionState.get(folderPath)}
                  <div
                    class={cn(
                      'group grid grid-cols-[minmax(0,1fr)_96px_78px_84px] gap-2 px-2.5 py-1.5 border-b border-border/60 items-center',
                      row.depth === 0 ? 'bg-muted/30' : 'bg-muted/10 hover:bg-muted/20'
                    )}
                  >
                    <button
                      onclick={() => toggleFolder(folderPath)}
                      class="flex items-center gap-1.5 min-w-0 text-left"
                      style="padding-left: {row.depth * 10}px"
                    >
                      <input
                        use:indeterminate={Boolean(select?.some)}
                        type="checkbox"
                        checked={Boolean(select?.all)}
                        disabled={!select ||
                          select.total === 0 ||
                          busy ||
                          deletingSelected ||
                          reloadingSelected}
                        onchange={event => {
                          event.stopPropagation();
                          toggleFolderSelection(folderPath, event.currentTarget.checked);
                        }}
                        onclick={event => event.stopPropagation()}
                        class="h-3.5 w-3.5 rounded-sm border border-primary/50 bg-background accent-primary disabled:opacity-50"
                        title="选择此文件夹中的文件(仅可见项)"
                        aria-label="选择文件夹文件"
                      />
                      <ChevronRight
                        size={12}
                        class={cn('transition-transform', row.isExpanded && 'rotate-90')}
                      />
                      {#if row.isExpanded}
                        <FolderOpen size={13} class="text-primary" />
                      {:else}
                        <Folder size={13} class="text-primary" />
                      {/if}
                      <span class="truncate text-[13px] font-medium">{row.name}</span>
                      <span
                        class="inline-flex items-center rounded px-1.5 py-0.5 text-[10px] border border-border bg-background/50 text-muted-foreground"
                        >{count}</span
                      >
                    </button>
                    <div class="text-[11px] text-muted-foreground">文件夹</div>
                    <div class="text-[11px] text-muted-foreground">-</div>
                    <div class="flex items-center justify-end">
                      <Button
                        onclick={event => {
                          event.stopPropagation();
                          handleDeleteFolder(folderPath);
                        }}
                        size="icon"
                        variant="outline"
                        class="h-6 w-6 text-stat-leecher border-stat-leecher/30 hover:bg-stat-leecher/10"
                        disabled={count === 0 || busy || reloadingSelected}
                        title="删除此文件夹中的全部文件"
                      >
                        {#snippet children()}
                          {#if busy}
                            <RotateCw size={11} class="animate-spin" />
                          {:else}
                            <Trash2 size={11} />
                          {/if}
                        {/snippet}
                      </Button>
                    </div>
                  </div>
                {:else}
                  {@const file = filteredMap.get(row.path)}
                  {@const StatusIcon = getStatusIcon(file?.status)}
                  {@const isSelected = selectedPaths.has(String(row.path))}
                  <div
                    class={cn(
                      'group grid grid-cols-[minmax(0,1fr)_96px_78px_84px] gap-2 px-2.5 py-1.5 border-b border-border/60 items-center hover:bg-muted/15',
                      isSelected && 'bg-primary/8'
                    )}
                    onclick={event => handleFileRowClick(event, row.path)}
                    onkeydown={event => handleFileRowKeydown(event, row.path)}
                    role="button"
                    tabindex="0"
                    aria-label={`Select file ${file?.name || row.name}`}
                  >
                    <div
                      class="flex items-center gap-1.5 min-w-0"
                      style="padding-left: {10 + row.depth * 10}px"
                    >
                      <input
                        type="checkbox"
                        checked={isSelected}
                        onchange={event =>
                          toggleFileSelected(
                            row.path,
                            event.currentTarget.checked,
                            Boolean(event.shiftKey)
                          )}
                        onclick={event => event.stopPropagation()}
                        onkeydown={event => event.stopPropagation()}
                        disabled={deletingSelected ||
                          reloadingSelected ||
                          deletingFile === row.path}
                        class="h-3.5 w-3.5 rounded-sm border border-primary/50 bg-background accent-primary disabled:opacity-50"
                        title="选择文件"
                        aria-label="选择文件"
                      />
                      <File size={12} class="text-muted-foreground" />
                      <div class="min-w-0">
                        <div class="truncate text-[13px] font-medium">{file?.name || row.name}</div>
                        <div
                          class="truncate text-[11px] text-muted-foreground/85 leading-tight opacity-70 group-hover:opacity-100"
                          title={row.path}
                        >
                          {row.path}
                        </div>
                      </div>
                    </div>
                    <div>
                      <span
                        class={cn(
                          'inline-flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded border',
                          getStatusBadge(file?.status)
                        )}
                      >
                        <StatusIcon size={10} />
                        {file?.status || 'unknown'}
                      </span>
                    </div>
                    <div class="text-[11px] text-muted-foreground">
                      {formatSize(file?.size || 0)}
                    </div>
                    <div class="flex items-center justify-end gap-1">
                      <Button
                        onclick={() => handleReloadFile(row.path)}
                        onkeydown={event => event.stopPropagation()}
                        disabled={reloadingSelected ||
                          deletingSelected ||
                          reloadingFile === row.path}
                        size="icon"
                        variant="outline"
                        class="h-6 w-6"
                        title="重新加载文件"
                      >
                        {#snippet children()}
                          <RotateCw
                            size={11}
                            class={cn(reloadingFile === row.path && 'animate-spin')}
                          />
                        {/snippet}
                      </Button>
                      <Button
                        onclick={() => handleDeleteFile(row.path)}
                        onkeydown={event => event.stopPropagation()}
                        disabled={reloadingSelected ||
                          deletingSelected ||
                          deletingFile === row.path}
                        size="icon"
                        variant="outline"
                        class="h-6 w-6 text-stat-leecher border-stat-leecher/30 hover:bg-stat-leecher/10"
                        title="删除文件"
                      >
                        {#snippet children()}
                          {#if deletingFile === row.path}
                            <X size={11} class="animate-pulse" />
                          {:else}
                            <Trash2 size={11} />
                          {/if}
                        {/snippet}
                      </Button>
                    </div>
                  </div>
                {/if}
              {/each}
            {/if}
          </div>
        </section>
      </div>
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:open={actionConfirmVisible}
  title={actionConfirmTitle}
  message={actionConfirmMessage}
  cancelLabel={actionConfirmCancelLabel}
  confirmLabel={actionConfirmOkLabel}
  kind={actionConfirmKind === 'warning' ? 'warning' : 'info'}
  titleId="watch-action-confirm-title"
  onCancel={() => resolveActionConfirm(false)}
  onConfirm={() => resolveActionConfirm(true)}
/>
