<script>
  import { instances, activeInstanceId, instanceActions } from '$lib/instanceStore.js';
  import { get } from 'svelte/store';
  import { api } from '$lib/api.js';
  import Button from '$lib/components/ui/button.svelte';
  import BaseModal from '../common/BaseModal.svelte';
  import { builtInPresets } from '$lib/presets/index.js';
  import {
    getDefaultPreset,
    getDefaultPresetId,
    setDefaultPreset,
    clearDefaultPreset,
    refreshDefaultPreset,
  } from '$lib/defaultPreset.js';
  import {
    buildCustomPreset,
    buildPresetExportData,
    normalizePreset,
    normalizePresets,
    normalizePresetSettings,
  } from '$lib/customPreset.js';
  import { THEMES, THEME_CATEGORIES, getTheme, selectTheme } from '$lib/themeStore.svelte.js';
  import { Settings, X, Check, Trash2, Download, Upload, Save } from '@lucide/svelte';
  import PresetIcon from '../config/PresetIcon.svelte';

  let { isOpen = $bindable(false) } = $props();

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  // Subscribe to stores for reactivity
  let currentInstances = $state([]);
  let currentActiveId = $state(null);

  instances.subscribe(value => (currentInstances = value));
  activeInstanceId.subscribe(value => (currentActiveId = value));

  // Tab state
  let activeTab = $state('general');

  // Log level state (stored in localStorage)
  const LOG_LEVEL_KEY = 'rustatio-log-level';
  let logLevel = $state(localStorage.getItem(LOG_LEVEL_KEY) || 'info');

  function saveLogLevel(level) {
    logLevel = level;
    localStorage.setItem(LOG_LEVEL_KEY, level);
    api.setLogLevel(level);
  }

  // Window Close Behavior state
  const CLOSE_BEHAVIOR_KEY = 'rustatio-close-behavior';
  function getCloseBehavior() {
    return localStorage.getItem(CLOSE_BEHAVIOR_KEY) || 'prompt';
  }

  let closeBehavior = $state(getCloseBehavior());

  function saveCloseBehavior(behavior) {
    closeBehavior = behavior;
    localStorage.setItem(CLOSE_BEHAVIOR_KEY, behavior);
  }

  $effect(() => {
    if (isOpen) {
      closeBehavior = getCloseBehavior();
    }
  });

  let customPresets = $state([]);

  // Default preset state
  let defaultPresetId = $state(getDefaultPresetId());
  let defaultPresetName = $state(getDefaultPreset()?.name || 'Rustatio 默认');

  async function loadPresetState() {
    try {
      customPresets = normalizePresets((await api.listCustomPresets()) || []);
      const preset = await refreshDefaultPreset();
      defaultPresetId = preset?.id || null;
      defaultPresetName = preset?.name || 'Rustatio 默认';
    } catch (e) {
      console.warn('Failed to load preset state:', e);
    }
  }

  async function setAsDefault(preset) {
    await setDefaultPreset(preset);
    defaultPresetId = preset.id;
    defaultPresetName = preset.name || '未命名预设';
  }

  async function clearDefault() {
    await clearDefaultPreset();
    defaultPresetId = null;
    defaultPresetName = 'Rustatio 默认';
  }

  $effect(() => {
    if (isOpen) {
      loadPresetState();
    }
  });

  // 全局上传上限(防封:所有运行实例目标速率之和的上限,0 = 不限)
  let globalMaxUpload = $state(0);
  let globalMaxUploadSaved = $state('');

  $effect(() => {
    if (isOpen) {
      loadGlobalMaxUpload();
    }
  });

  async function loadGlobalMaxUpload() {
    try {
      const config = await api.getConfig();
      globalMaxUpload = Math.round(config?.faker?.global_max_upload ?? 0);
    } catch (e) {
      console.warn('Failed to load global max upload:', e);
    }
  }

  async function saveGlobalMaxUpload() {
    try {
      const value = Math.max(0, Math.round(globalMaxUpload || 0));
      const config = await api.getConfig();
      config.faker = { ...config.faker, global_max_upload: value };
      await api.updateConfig(config);
      globalMaxUpload = value;
      globalMaxUploadSaved = value > 0 ? `已保存:全部运行实例总上传不超过 ${value} KB/s` : '已保存:不限速';
      setTimeout(() => (globalMaxUploadSaved = ''), 5000);
    } catch (e) {
      globalMaxUploadSaved = `保存失败: ${e}`;
    }
  }

  // Detection avoidance tips
  const detectionTips = [
    {
      title: '使用 VPN',
      description: '始终使用 VPN 隐藏真实 IP。Tracker 会将你的 IP 与多个种子关联,从而发现异常。',
      importance: 'critical',
    },
    {
      title: '与历史记录保持一致',
      description:
        '如果你一直用 qBittorrent,不要突然换成 uTorrent。坚持使用你历史上一直在用的客户端。',
      importance: 'high',
    },
    {
      title: '速率要真实',
      description:
        '上传速率不要超过你实际带宽的上限。10 Mbps 的宽带不可能以 50 MB/s 做种。',
      importance: 'high',
    },
    {
      title: '开启速率随机化',
      description:
        '真实的种子传输速度是波动的,恒定速率非常可疑。请始终开启速率随机化。',
      importance: 'high',
    },
    {
      title: '使用渐进速率',
      description:
        '真实用户是逐渐互相发现的,一开始就全速跑很不自然。建议开启渐进速率调整。',
      importance: 'medium',
    },
    {
      title: '避免整数速率',
      description:
        '恰好 100 KB/s 或 50 KB/s 这种整数速率很可疑。随机化功能可以避免这一点,也可以把基础速率设成 47 或 103 KB/s 这类数值。',
      importance: 'medium',
    },
    {
      title: '完成度要匹配',
      description:
        '伪装下载时,请合理设置完成百分比。一边大量上传一边报告 0% 下载进度是很可疑的。',
      importance: 'medium',
    },
    {
      title: '不要贪多',
      description:
        '细水长流地涨分享率,比一天冲到 10 倍安全得多。请设置停止条件来限制单次会话的分享率。',
      importance: 'medium',
    },
  ];

  function close() {
    isOpen = false;
  }

  function applyPreset(preset) {
    const active = get(activeInstanceId);
    if (active !== null) {
      instanceActions.updateInstance(active, normalizePresetSettings(preset.settings));
    }
    close();
  }

  // Check if a preset matches the current instance settings
  function isPresetApplied(preset, instance) {
    if (!instance) return false;

    // Compare all settings in the preset with the instance
    for (const [key, value] of Object.entries(normalizePresetSettings(preset.settings))) {
      // Handle numeric comparisons with tolerance for floating point
      if (typeof value === 'number' && typeof instance[key] === 'number') {
        if (Math.abs(instance[key] - value) > 0.001) return false;
      } else if (instance[key] !== value) {
        return false;
      }
    }
    return true;
  }

  // Reactive check for applied presets - updates when instances or customPresets change
  let appliedPresetId = $derived.by(() => {
    // Access all reactive dependencies explicitly
    const instances = currentInstances;
    const activeId = currentActiveId;
    const custom = customPresets; // Must access to make reactive

    if (!instances || activeId === null) return null;

    const instance = instances.find(i => i.id === activeId);
    if (!instance) return null;

    // Check built-in presets
    for (const preset of builtInPresets) {
      if (isPresetApplied(preset, instance)) return preset.id;
    }
    // Check custom presets
    for (const preset of custom) {
      if (isPresetApplied(preset, instance)) return preset.id;
    }
    return null;
  });

  // Export current config as a custom preset
  let exportError = $state('');
  let exportSuccess = $state('');
  let showExportDialog = $state(false);
  let showSaveDialog = $state(false);
  let exportPresetName = $state('');
  let exportPresetDescription = $state('');

  function getActiveInstanceOrThrow() {
    const active = get(activeInstanceId);
    if (active === null) {
    throw new Error('没有活动实例,请先选择一个实例。');
    }

    const currentItems = get(instances);
    const instance = currentItems.find(i => i.id === active);
    if (!instance) {
      throw new Error('未找到实例。');
    }

    return instance;
  }

  function resetPresetDialog() {
    exportError = '';
    exportSuccess = '';
    exportPresetName = '';
    exportPresetDescription = '';
  }

  function openSaveDialog() {
    resetPresetDialog();

    try {
      getActiveInstanceOrThrow();
      showSaveDialog = true;
    } catch (err) {
      exportError = err.message;
    }
  }

  function openExportDialog() {
    resetPresetDialog();

    try {
      getActiveInstanceOrThrow();
      showExportDialog = true;
    } catch (err) {
      exportError = err.message;
    }
  }

  async function savePreset(setDefault = false) {
    exportError = '';
    exportSuccess = '';

    try {
      const instance = getActiveInstanceOrThrow();
      const preset = buildCustomPreset(instance, {
        name: exportPresetName,
        description: exportPresetDescription,
      });

      await api.upsertCustomPreset(preset);
      customPresets = [...customPresets.filter(p => p.id !== preset.id), preset];

      if (setDefault) {
        await setAsDefault(preset);
      }

      exportSuccess = setDefault
        ? `预设"${preset.name}"已保存并设为默认`
        : `预设"${preset.name}"已保存`;
      showSaveDialog = false;
    } catch (err) {
      exportError = err.message;
    }
  }

  async function exportPreset() {
    exportError = '';
    exportSuccess = '';
    let presetData;

    try {
      const instance = getActiveInstanceOrThrow();
      presetData = buildPresetExportData(instance, {
        name: exportPresetName,
        description: exportPresetDescription,
      });
    } catch (err) {
      exportError = err.message;
      return;
    }

    // Create a safe filename from the preset name
    const safeFilename = presetData.name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '');
    const defaultFilename = `rustatio-preset-${safeFilename || 'custom'}.json`;
    const jsonString = JSON.stringify(presetData, null, 2);

    if (isTauri) {
      // Use Tauri save dialog + write_file command
      try {
        const { save } = await import('@tauri-apps/plugin-dialog');
        const filePath = await save({
          defaultPath: defaultFilename,
          filters: [{ name: 'JSON', extensions: ['json'] }],
        });

        if (filePath) {
          const { invoke } = await import('@tauri-apps/api/core');
          await invoke('write_file', { path: filePath, contents: jsonString });
          exportSuccess = '配置导出成功';
          showExportDialog = false;
        }
      } catch (err) {
        console.error('Export failed:', err);
        exportError = `导出失败:${err.message}`;
      }
    } else {
      // Browser: use download with suggested filename
      try {
        const blob = new Blob([jsonString], { type: 'application/json' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = defaultFilename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        exportSuccess = '配置导出成功';
        showExportDialog = false;
      } catch (err) {
        console.error('Export failed:', err);
        exportError = `导出失败:${err.message}`;
      }
    }
  }

  // Import preset from file
  let fileInput = $state(null);
  let importError = $state('');
  let importSuccess = $state('');

  function triggerImport() {
    fileInput?.click();
  }

  async function handleFileImport(event) {
    const file = event.target.files?.[0];
    if (!file) return;

    importError = '';
    importSuccess = '';

    try {
      const text = await file.text();
      const data = JSON.parse(text);

      // Validate preset structure
      if (data.type !== 'rustatio-preset' || !data.settings) {
        throw new Error('无效的预设文件格式');
      }

      // Create custom preset object
      const newPreset = normalizePreset({
        id: `custom-${Date.now()}`,
        name: data.name || '导入的预设',
        description: data.description || '导入的自定义预设',
        icon: data.icon || 'folder',
        custom: true,
        created_at: data.created_at || data.createdAt || new Date().toISOString(),
        settings: data.settings,
      });

      // Add to custom presets
      await api.upsertCustomPreset(newPreset);
      customPresets = [...customPresets.filter(p => p.id !== newPreset.id), newPreset];

      importSuccess = `预设"${newPreset.name}"导入成功`;
    } catch (err) {
      importError = `导入失败:${err.message}`;
    }

    // Reset file input
    if (fileInput) fileInput.value = '';
  }

  async function deleteCustomPreset(presetId) {
    await api.deleteCustomPreset(presetId);
    customPresets = customPresets.filter(p => p.id !== presetId);
    if (defaultPresetId === presetId) {
      await clearDefault();
    }
  }

  function getImportanceColor(importance) {
    switch (importance) {
      case 'critical':
        return 'text-stat-leecher bg-stat-leecher/10';
      case 'high':
        return 'text-stat-ratio bg-stat-ratio/10';
      case 'medium':
        return 'text-blue-500 bg-blue-500/10';
      default:
        return 'text-muted-foreground bg-muted';
    }
  }

  function getImportanceLabel(importance) {
    switch (importance) {
      case 'critical':
        return '严重';
      case 'high':
        return '重要';
      case 'medium':
        return '建议';
      default:
        return '提示';
    }
  }
</script>

{#if isOpen}
  <BaseModal
    bind:open={isOpen}
    onClose={close}
    titleId="settings-title"
    maxWidthClass="max-w-2xl"
    panelClass="max-h-[85vh] flex flex-col animate-in fade-in zoom-in-95 duration-200"
  >
    <!-- Header -->
    <div class="flex items-start justify-between p-6 border-b border-border flex-shrink-0">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center">
          <Settings size={20} class="text-primary" />
        </div>
        <div>
          <h2 id="settings-title" class="text-xl font-bold text-foreground">设置</h2>
          <p class="text-sm text-muted-foreground">预设与配置</p>
        </div>
      </div>
      <button
        onclick={close}
        class="p-1 rounded hover:bg-muted transition-colors"
        aria-label="关闭对话框"
      >
        <X size={20} />
      </button>
    </div>

    <!-- Tabs -->
    <div class="flex border-b border-border flex-shrink-0">
      <button
        class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'general'
          ? 'text-primary border-b-2 border-primary bg-primary/5'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => (activeTab = 'general')}
      >常规</button>
      <button
        class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'presets'
          ? 'text-primary border-b-2 border-primary bg-primary/5'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => (activeTab = 'presets')}
      >预设</button>
      <button
        class="flex-1 px-4 py-3 text-sm font-medium transition-colors {activeTab === 'tips'
          ? 'text-primary border-b-2 border-primary bg-primary/5'
          : 'text-muted-foreground hover:text-foreground'}"
        onclick={() => (activeTab = 'tips')}
      >检测提示</button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      {#if activeTab === 'general'}
        <!-- General Settings Tab -->
        <div class="space-y-6">
          <p class="text-sm text-muted-foreground mb-4">配置应用常规设置。</p>

          <!-- Log Level Section -->
          <div class="border border-border rounded-lg p-4">
            <h3 class="font-semibold text-foreground mb-2">日志级别</h3>
            <p class="text-sm text-muted-foreground mb-4">
              设置控制台中显示的日志详细程度。级别越高,调试时能看到的信息越详细。
            </p>
            <div class="flex items-center gap-4">
              <label for="logLevel" class="text-sm font-medium min-w-[60px]">级别</label>
              <select
                id="logLevel"
                value={logLevel}
                onchange={e => saveLogLevel(e.target.value)}
                class="px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 w-40"
              >
                <option value="error">错误</option>
                <option value="warn">警告</option>
                <option value="info">信息</option>
                <option value="debug">调试</option>
                <option value="trace">跟踪</option>
              </select>
            </div>
            <div class="mt-3 text-xs text-muted-foreground space-y-1">
              <p><strong>错误:</strong>仅关键错误</p>
              <p><strong>警告:</strong>错误与警告</p>
              <p><strong>信息:</strong>一般信息(默认)</p>
              <p><strong>调试:</strong>详细调试信息</p>
              <p><strong>跟踪:</strong>非常详细,包含全部内部操作</p>
            </div>
            <p class="mt-3 text-xs text-muted-foreground italic">注意:日志级别更改对后端生效。</p>
          </div>

          <!-- Window Behavior Section (Tauri only) -->
          {#if isTauri}
            <div class="border border-border rounded-lg p-4">
              <h3 class="font-semibold text-foreground mb-2">窗口行为</h3>
              <p class="text-sm text-muted-foreground mb-4">
                选择点击窗口关闭按钮时执行的操作。
              </p>
              <div class="flex items-center gap-4">
                <label for="closeBehavior" class="text-sm font-medium min-w-[60px]">关闭时</label>
                <select
                  id="closeBehavior"
                  value={closeBehavior}
                  onchange={e => saveCloseBehavior(e.target.value)}
                  class="px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 w-56"
                >
                  <option value="prompt">每次询问</option>
                  <option value="tray">最小化到托盘</option>
                  <option value="quit">退出程序</option>
                </select>
              </div>
              <div class="mt-3 text-xs text-muted-foreground space-y-1">
                <p><strong>每次询问:</strong>显示提示以选择操作</p>
                <p><strong>最小化到托盘:</strong>应用保持在后台运行</p>
                <p><strong>退出程序:</strong>完全关闭应用</p>
              </div>
            </div>
          {/if}

          <!-- 全局速率上限(防封) -->
          <div class="border border-border rounded-lg p-4">
            <h3 class="font-semibold text-foreground mb-2">全局上传上限(防封)</h3>
            <p class="text-sm text-muted-foreground mb-3">
              所有运行实例的目标上传速率之和超过此值时,自动按比例下调,防止 tracker 看到物理上不可能的全局速率。0 = 不限。
            </p>
            <div class="flex items-center gap-3">
              <label for="globalMaxUpload" class="text-sm font-medium min-w-[60px]">上限</label>
              <input
                id="globalMaxUpload"
                type="number"
                bind:value={globalMaxUpload}
                min="0"
                step="100"
                class="px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 w-40"
              />
              <span class="text-sm text-muted-foreground">KB/s</span>
              <button
                type="button"
                class="h-8 rounded-md bg-primary px-3 text-xs font-semibold text-primary-foreground hover:bg-primary/90 cursor-pointer"
                onclick={saveGlobalMaxUpload}
              >保存</button>
            </div>
            {#if globalMaxUploadSaved}
              <p class="mt-2 text-xs text-stat-upload">{globalMaxUploadSaved}</p>
            {/if}
          </div>

          <!-- Theme Section -->
          <div class="border border-border rounded-lg p-4">
            <h3 class="font-semibold text-foreground mb-2">主题</h3>
            <p class="text-sm text-muted-foreground mb-4">选择喜欢的配色主题。</p>
            <div class="flex items-center gap-4">
              <label for="themeSelect" class="text-sm font-medium min-w-[60px]">主题</label>
              <select
                id="themeSelect"
                value={getTheme()}
                onchange={e => selectTheme(e.target.value)}
                class="px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 w-56"
              >
                {#each Object.entries(THEME_CATEGORIES) as [categoryId, category] (categoryId)}
                  <optgroup label={category.name}>
                    {#each category.themes as themeId (themeId)}
                      {@const themeOption = THEMES[themeId]}
                      <option value={themeOption.id}>{themeOption.name}</option>
                    {/each}
                  </optgroup>
                {/each}
              </select>
            </div>
            <p class="mt-3 text-xs text-muted-foreground">
              {THEMES[getTheme()]?.description || ''}
            </p>
          </div>
        </div>
      {:else if activeTab === 'presets'}
        <!-- Presets Tab -->
        <div class="space-y-6">
          <!-- Info box about Apply vs Default -->
          <div class="bg-muted/50 border border-border rounded-lg p-4">
            <p class="text-sm text-muted-foreground">
              <span class="font-semibold text-foreground">应用</span>仅把预设套用到当前实例。
              <span class="font-semibold text-foreground">设为默认</span>后,新建的实例/种子会自动使用该预设的设置。
            </p>
            <p class="text-xs text-muted-foreground mt-2">监视文件夹导入使用的默认值:<span class="text-foreground font-medium">{defaultPresetName}</span>
            </p>
          </div>

          <!-- Built-in Presets -->
          <div>
            <h3 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-3">内置预设</h3>
            <div class="space-y-3">
              {#each builtInPresets as preset (preset.id)}
                <div
                  class="border border-border rounded-lg p-4 hover:border-primary/50 transition-colors {preset.recommended
                    ? 'ring-1 ring-primary/30'
                    : ''}"
                >
                  <!-- Header row with title and action button -->
                  <div class="flex items-start justify-between gap-3 mb-2">
                    <div class="flex items-center gap-2 flex-wrap flex-1 min-w-0">
                      <PresetIcon icon={preset.icon} size={20} class="flex-shrink-0 text-primary" />
                      <h3 class="font-semibold text-foreground">{preset.name}</h3>
                      {#if preset.recommended}
                        <span
                          class="text-xs px-2 py-0.5 rounded-full bg-primary/20 text-primary font-medium"
                        >推荐</span>
                      {/if}
                    </div>
                    <!-- Action buttons in header -->
                    <div class="flex items-center gap-1 flex-shrink-0">
                      {#if appliedPresetId === preset.id}
                        <span
                          class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-lg bg-stat-upload/20 text-stat-upload"
                        >
                          <Check size={14} strokeWidth={2.5} />已应用</span>
                      {:else}
                        <Button size="sm" onclick={() => applyPreset(preset)}>应用</Button>
                      {/if}
                      {#if defaultPresetId === preset.id}
                        <button
                          onclick={() => clearDefault()}
                          class="ml-1 px-2 py-1.5 text-xs font-medium rounded-lg bg-primary/20 text-primary hover:bg-primary/30 transition-colors"
                          title="点击清除默认"
                        >
                          ★ 默认
                        </button>
                      {:else}
                        <button
                          onclick={() => setAsDefault(preset)}
                          class="ml-1 px-2 py-1.5 text-xs font-medium rounded-lg border border-border hover:bg-muted transition-colors"
                          title="设为新实例默认"
                        >设为默认</button>
                      {/if}
                    </div>
                  </div>

                  <p class="text-sm text-muted-foreground mb-3">{preset.description}</p>

                  <!-- Settings preview -->
                  <div class="flex flex-wrap gap-2 text-xs mb-2">
                    <span class="px-2 py-1 bg-muted rounded"
                      >↑ {preset.settings.uploadRate} KB/s</span
                    >
                    <span class="px-2 py-1 bg-muted rounded"
                      >↓ {preset.settings.downloadRate} KB/s</span
                    >
                    {#if preset.settings.randomizeRates}
                      <span class="px-2 py-1 bg-muted rounded"
                        >±{preset.settings.randomRangePercent}%</span
                      >
                    {/if}
                    {#if preset.settings.progressiveRatesEnabled}
                      <span class="px-2 py-1 bg-stat-upload/20 text-stat-upload rounded"
                        >渐变</span
                      >
                    {/if}
                    {#if preset.settings.selectedClient}
                      <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded capitalize"
                        >{preset.settings.selectedClient}</span
                      >
                    {/if}
                    <!-- Stop conditions -->
                    {#if preset.settings.stopAtRatioEnabled}
                      <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                        >达到 {preset.settings.stopAtRatio}x 停止</span
                      >
                    {/if}
                    {#if preset.settings.stopAtUploadedEnabled}
                      <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                        >上传达 {preset.settings.stopAtUploadedGB} GB 停止</span
                      >
                    {/if}
                    {#if preset.settings.stopAtDownloadedEnabled}
                      <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                        >下载达 {preset.settings.stopAtDownloadedGB} GB 停止</span
                      >
                    {/if}
                    {#if preset.settings.stopAtSeedTimeEnabled}
                      <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                        >做种 {preset.settings.stopAtSeedTimeHours} 小时停止</span
                      >
                    {/if}
                    {#if preset.settings.idleWhenNoLeechers}
                      <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded"
                        >空闲:无下载者</span
                      >
                    {/if}
                    {#if preset.settings.idleWhenNoSeeders}
                      <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                        >空闲:无做种者</span
                      >
                    {/if}
                  </div>

                  <!-- Tips -->
                  <details class="text-xs">
                    <summary
                      class="cursor-pointer text-muted-foreground hover:text-foreground transition-colors"
                    >
                      为什么这样设置?
                    </summary>
                    <ul class="mt-2 space-y-1 text-muted-foreground pl-4">
                      {#each preset.tips as tip, tipIndex (tipIndex)}
                        <li class="list-disc">{tip}</li>
                      {/each}
                    </ul>
                  </details>
                </div>
              {/each}
            </div>
          </div>

          <!-- Custom Presets -->
          <div>
            <h3 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-3">自定义预设</h3>

            {#if customPresets.length > 0}
              <div class="space-y-3 mb-4">
                {#each customPresets as preset (preset.id)}
                  <div
                    class="border border-border rounded-lg p-4 hover:border-primary/50 transition-colors"
                  >
                    <!-- Header row with title and action buttons -->
                    <div class="flex items-start justify-between gap-3 mb-2">
                      <div class="flex items-center gap-2 flex-wrap flex-1 min-w-0">
                        <PresetIcon
                          icon={preset.icon}
                          size={20}
                          class="flex-shrink-0 text-primary"
                        />
                        <h3 class="font-semibold text-foreground">{preset.name}</h3>
                        <span
                          class="text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground"
                        >自定义</span>
                      </div>
                      <!-- Action buttons in header -->
                      <div class="flex items-center gap-1 flex-shrink-0">
                        {#if appliedPresetId === preset.id}
                          <span
                            class="inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-lg bg-stat-upload/20 text-stat-upload"
                          >
                            <Check size={14} strokeWidth={2.5} />已应用</span>
                        {:else}
                          <Button size="sm" onclick={() => applyPreset(preset)}>应用</Button>
                        {/if}
                        {#if defaultPresetId === preset.id}
                          <button
                            onclick={() => clearDefault()}
                            class="px-2 py-1.5 text-xs font-medium rounded-lg bg-primary/20 text-primary hover:bg-primary/30 transition-colors"
                            title="点击清除默认"
                          >
                            ★ 默认
                          </button>
                        {:else}
                          <button
                            onclick={() => setAsDefault(preset)}
                            class="px-2 py-1.5 text-xs font-medium rounded-lg border border-border hover:bg-muted transition-colors"
                            title="设为新实例默认"
                          >设为默认</button>
                        {/if}
                        <button
                          onclick={() => deleteCustomPreset(preset.id)}
                          class="p-2 rounded hover:bg-stat-leecher/10 text-muted-foreground hover:text-stat-leecher transition-colors"
                          aria-label="删除预设"
                        >
                          <Trash2 size={16} />
                        </button>
                      </div>
                    </div>

                    <p class="text-sm text-muted-foreground mb-3">{preset.description}</p>

                    <!-- Settings preview -->
                    <div class="flex flex-wrap gap-2 text-xs">
                      <span class="px-2 py-1 bg-muted rounded"
                        >↑ {preset.settings.uploadRate} KB/s</span
                      >
                      <span class="px-2 py-1 bg-muted rounded"
                        >↓ {preset.settings.downloadRate} KB/s</span
                      >
                      {#if preset.settings.randomizeRates}
                        <span class="px-2 py-1 bg-muted rounded"
                          >±{preset.settings.randomRangePercent}%</span
                        >
                      {/if}
                      {#if preset.settings.progressiveRatesEnabled}
                        <span class="px-2 py-1 bg-stat-upload/20 text-stat-upload rounded"
                          >渐变</span
                        >
                      {/if}
                      {#if preset.settings.selectedClient}
                        <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded capitalize"
                          >{preset.settings.selectedClient}</span
                        >
                      {/if}
                      <!-- Stop conditions -->
                      {#if preset.settings.stopAtRatioEnabled}
                        <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                          >达到 {preset.settings.stopAtRatio}x 停止</span
                        >
                      {/if}
                      {#if preset.settings.stopAtUploadedEnabled}
                        <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                          >上传达 {preset.settings.stopAtUploadedGB} GB 停止</span
                        >
                      {/if}
                      {#if preset.settings.stopAtDownloadedEnabled}
                        <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                          >下载达 {preset.settings.stopAtDownloadedGB} GB 停止</span
                        >
                      {/if}
                      {#if preset.settings.stopAtSeedTimeEnabled}
                        <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                          >做种 {preset.settings.stopAtSeedTimeHours} 小时停止</span
                        >
                      {/if}
                      {#if preset.settings.idleWhenNoLeechers}
                        <span class="px-2 py-1 bg-purple-500/20 text-purple-500 rounded"
                          >空闲:无下载者</span
                        >
                      {/if}
                      {#if preset.settings.idleWhenNoSeeders}
                        <span class="px-2 py-1 bg-orange-500/20 text-orange-500 rounded"
                          >空闲:无做种者</span
                        >
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="text-sm text-muted-foreground mb-4">
                暂无自定义预设。可将当前实例保存为预设,或导入预设文件。
              </p>
            {/if}

            <!-- Import/Export Section -->
            <div class="border border-dashed border-border rounded-lg p-4 space-y-4">
              <!-- Save current config -->
              <div>
                <h4 class="font-medium text-foreground mb-2">保存当前为预设</h4>
                <p class="text-sm text-muted-foreground mb-3">
                  将当前活动实例的配置直接保存到自定义预设。
                </p>
                <button
                  type="button"
                  onclick={openSaveDialog}
                  class="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold ring-offset-background transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 bg-primary text-primary-foreground shadow-lg shadow-primary/25 hover:bg-primary/90 hover:shadow-xl hover:shadow-primary/30 hover:-translate-y-0.5 active:scale-95 px-4 py-2 text-sm"
                >
                  <Save size={16} />保存预设</button>
                {#if exportSuccess}
                  <p class="mt-2 text-sm text-stat-upload">{exportSuccess}</p>
                {/if}
              </div>

              <!-- Export current config -->
              <div class="border-t border-border pt-4">
                <h4 class="font-medium text-foreground mb-2">导出当前配置</h4>
                <p class="text-sm text-muted-foreground mb-3">
                  将当前配置保存为 JSON 文件,便于分享和再次导入。
                </p>
                <button
                  type="button"
                  onclick={openExportDialog}
                  class="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold ring-offset-background transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 bg-primary text-primary-foreground shadow-lg shadow-primary/25 hover:bg-primary/90 hover:shadow-xl hover:shadow-primary/30 hover:-translate-y-0.5 active:scale-95 px-4 py-2 text-sm"
                >
                  <Download size={16} />导出配置</button>
              </div>

              <!-- Import preset -->
              <div class="border-t border-border pt-4">
                <h4 class="font-medium text-foreground mb-2">导入预设文件</h4>
                <input
                  bind:this={fileInput}
                  type="file"
                  accept=".json"
                  class="hidden"
                  onchange={handleFileImport}
                />
                <button
                  type="button"
                  onclick={triggerImport}
                  class="inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg font-semibold ring-offset-background transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 border-2 border-primary/20 bg-background hover:bg-primary/5 hover:border-primary/40 hover:-translate-y-0.5 active:scale-95 px-4 py-2 text-sm"
                >
                  <Upload size={16} />导入预设</button>
                {#if importError}
                  <p class="mt-2 text-sm text-stat-leecher">{importError}</p>
                {/if}
                {#if importSuccess}
                  <p class="mt-2 text-sm text-stat-upload">{importSuccess}</p>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {:else if activeTab === 'tips'}
        <!-- Detection Tips Tab -->
        <div class="space-y-4">
          <p class="text-sm text-muted-foreground mb-4">
            遵循以下准则,降低被私人 tracker 检测到的风险。
          </p>

          {#each detectionTips as tip, index (index)}
            <div class="border border-border rounded-lg p-4">
              <div class="flex flex-col gap-2">
                <div class="flex items-center justify-between gap-3">
                  <h3 class="font-semibold text-foreground">{tip.title}</h3>
                  <span
                    class="flex-shrink-0 text-xs font-semibold px-2 py-1 rounded {getImportanceColor(
                      tip.importance
                    )}"
                  >
                    {getImportanceLabel(tip.importance)}
                  </span>
                </div>
                <p class="text-sm text-muted-foreground">{tip.description}</p>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </BaseModal>
{/if}

<!-- Save Preset Dialog -->
{#if showSaveDialog}
  <BaseModal
    bind:open={showSaveDialog}
    onClose={() => (showSaveDialog = false)}
    titleId="save-dialog-title"
    maxWidthClass="max-w-md"
    panelClass="animate-in fade-in zoom-in-95 duration-200"
  >
    <div class="flex items-center justify-between p-4 border-b border-border">
      <h3 id="save-dialog-title" class="text-lg font-semibold text-foreground">保存预设</h3>
      <button
        onclick={() => (showSaveDialog = false)}
        class="p-1 rounded hover:bg-muted transition-colors"
        aria-label="关闭对话框"
      >
        <X size={18} />
      </button>
    </div>

    <div class="p-4 space-y-4">
      <div>
        <label for="save-preset-name" class="block text-sm font-medium text-foreground mb-1">预设名称<span class="text-stat-leecher">*</span>
        </label>
        <input
          id="save-preset-name"
          type="text"
          bind:value={exportPresetName}
          placeholder="例如:我的 Tracker 配置"
          class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
        />
      </div>

      <div>
        <label for="save-preset-description" class="block text-sm font-medium text-foreground mb-1">描述<span class="text-muted-foreground text-xs">(可选)</span>
        </label>
        <textarea
          id="save-preset-description"
          bind:value={exportPresetDescription}
          placeholder="描述这个预设的用途..."
          rows="2"
          class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 resize-none"
        ></textarea>
      </div>

      {#if exportError}
        <p class="text-sm text-stat-leecher">{exportError}</p>
      {/if}
    </div>

    <div class="flex justify-end gap-3 p-4 border-t border-border">
      <button
        type="button"
        onclick={() => (showSaveDialog = false)}
        class="px-4 py-2 text-sm font-medium rounded-lg border border-border hover:bg-muted transition-colors"
      >取消</button>
      <button
        type="button"
        onclick={() => savePreset(false)}
        class="px-4 py-2 text-sm font-medium rounded-lg border border-border hover:bg-muted transition-colors"
      >保存</button>
      <button
        type="button"
        onclick={() => savePreset(true)}
        class="px-4 py-2 text-sm font-semibold rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
      >保存并设为默认</button>
    </div>
  </BaseModal>
{/if}

<!-- Export Preset Dialog -->
{#if showExportDialog}
  <BaseModal
    bind:open={showExportDialog}
    onClose={() => (showExportDialog = false)}
    titleId="export-dialog-title"
    maxWidthClass="max-w-md"
    panelClass="animate-in fade-in zoom-in-95 duration-200"
  >
    <!-- Header -->
    <div class="flex items-center justify-between p-4 border-b border-border">
      <h3 id="export-dialog-title" class="text-lg font-semibold text-foreground">导出预设</h3>
      <button
        onclick={() => (showExportDialog = false)}
        class="p-1 rounded hover:bg-muted transition-colors"
        aria-label="关闭对话框"
      >
        <X size={18} />
      </button>
    </div>

    <!-- Content -->
    <div class="p-4 space-y-4">
      <div>
        <label for="preset-name" class="block text-sm font-medium text-foreground mb-1">预设名称<span class="text-stat-leecher">*</span>
        </label>
        <input
          id="preset-name"
          type="text"
          bind:value={exportPresetName}
          placeholder="例如:我的 Tracker 配置"
          class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50"
        />
      </div>

      <div>
        <label for="preset-description" class="block text-sm font-medium text-foreground mb-1">描述<span class="text-muted-foreground text-xs">(可选)</span>
        </label>
        <textarea
          id="preset-description"
          bind:value={exportPresetDescription}
          placeholder="描述这个预设的用途..."
          rows="2"
          class="w-full px-3 py-2 text-sm border border-border rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 resize-none"
        ></textarea>
      </div>

      {#if exportError}
        <p class="text-sm text-stat-leecher">{exportError}</p>
      {/if}
    </div>

    <!-- Footer -->
    <div class="flex justify-end gap-3 p-4 border-t border-border">
      <button
        type="button"
        onclick={() => (showExportDialog = false)}
        class="px-4 py-2 text-sm font-medium rounded-lg border border-border hover:bg-muted transition-colors"
      >取消</button>
      <button
        type="button"
        onclick={exportPreset}
        class="px-4 py-2 text-sm font-semibold rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
      >导出</button>
    </div>
  </BaseModal>
{/if}
