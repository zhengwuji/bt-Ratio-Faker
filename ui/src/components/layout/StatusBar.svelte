<script>
  import { cn } from '$lib/utils.js';
  import { Play, Pause, Square, RefreshCw, Rocket, Moon, CircleCheck } from '@lucide/svelte';

  let {
    statusMessage,
    statusType,
    statusIcon = null,
    embedded = false,
    isRunning = false,
    isPaused = false,
    announceFeedback = null, // { count, seeders, leechers } 运行中正常汇报反馈
    onShowPeers = null,
    startFaking = null,
    stopFaking = null,
    pauseFaking = null,
    resumeFaking = null,
    manualUpdate = null,
  } = $props();

  const statusMeta = {
    idle: {
      label: '就绪',
      frame: 'border-stat-upload/20',
      chip: 'border-stat-upload/25 bg-stat-upload/10 text-stat-upload',
      dot: 'bg-stat-upload',
    },
    running: {
      label: '运行中',
      frame: 'border-primary/20',
      chip: 'border-primary/25 bg-primary/10 text-primary',
      dot: 'bg-primary',
    },
    paused: {
      label: '已暂停',
      frame: 'border-stat-ratio/20',
      chip: 'border-stat-ratio/25 bg-stat-ratio/10 text-stat-ratio',
      dot: 'bg-stat-ratio',
    },
    idling: {
      label: '空闲中',
      frame: 'border-stat-ratio/20',
      chip: 'border-stat-ratio/25 bg-stat-ratio/10 text-stat-ratio',
      dot: 'bg-stat-ratio',
    },
    success: {
      label: '已完成',
      frame: 'border-stat-upload/20',
      chip: 'border-stat-upload/25 bg-stat-upload/10 text-stat-upload',
      dot: 'bg-stat-upload',
    },
    warning: {
      label: '警告',
      frame: 'border-stat-ratio/20',
      chip: 'border-stat-ratio/25 bg-stat-ratio/10 text-stat-ratio',
      dot: 'bg-stat-ratio',
    },
    error: {
      label: '错误',
      frame: 'border-destructive/20',
      chip: 'border-destructive/25 bg-destructive/10 text-destructive',
      dot: 'bg-destructive',
    },
  };

  const actionBase =
    'h-8 inline-flex items-center gap-1.5 rounded-full border px-3 text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 cursor-pointer';

  const embeddedActionBase =
    'inline-flex h-8 w-8 items-center justify-center rounded-xl border transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 cursor-pointer';

  function getStatusMeta(type) {
    return statusMeta[type] || statusMeta.idle;
  }
</script>

<div
  class={cn(
    embedded
      ? 'bg-transparent px-0 py-0 shadow-none transition-colors'
      : 'rounded-2xl border bg-card/80 px-3 py-3 shadow-sm backdrop-blur-sm transition-colors',
    !embedded && getStatusMeta(statusType).frame
  )}
  role="status"
  aria-live="polite"
>
  <div
    class={cn(
      embedded
        ? 'flex min-w-0 items-center justify-between gap-3'
        : 'flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between'
    )}
  >
    <div
      class={cn(
        'min-w-0',
        embedded
          ? 'flex flex-1 items-center gap-3'
          : 'flex flex-col gap-2 lg:flex-row lg:items-center lg:gap-3'
      )}
    >
      <span
        class={cn(
          embedded
            ? 'inline-flex h-7 w-fit items-center gap-2 rounded-full border px-2.5 text-[10px] font-bold uppercase tracking-[0.24em]'
            : 'inline-flex w-fit items-center gap-2 rounded-full border px-3 py-1 text-[11px] font-bold uppercase tracking-[0.18em]',
          getStatusMeta(statusType).chip
        )}
      >
        {#if statusIcon === 'rocket'}
          <Rocket size={13} class="flex-shrink-0" />
        {:else if statusIcon === 'moon'}
          <Moon size={13} class="flex-shrink-0" />
        {:else if statusIcon === 'pause'}
          <Pause size={13} class="flex-shrink-0" fill="currentColor" />
        {:else}
          <span
            class={cn(
              'h-2 w-2 rounded-full flex-shrink-0',
              getStatusMeta(statusType).dot,
              statusType === 'running' && 'animate-pulse'
            )}
          ></span>
        {/if}
        <span>{getStatusMeta(statusType).label}</span>
      </span>

      <p
        class={cn(
          'min-w-0 leading-5',
          embedded
            ? 'truncate text-sm font-medium text-foreground/80'
            : 'text-sm font-medium text-foreground/90'
        )}
      >
        {statusMessage}
      </p>

      {#if isRunning && !isPaused && announceFeedback}
        <button
          type="button"
          class="inline-flex w-fit flex-shrink-0 items-center gap-1.5 rounded-full border border-stat-upload/25 bg-stat-upload/10 px-2.5 py-1 text-[11px] font-semibold text-stat-upload hover:bg-stat-upload/20 transition-colors cursor-pointer"
          title="汇报正常。点击查看正在做种的 Peer 列表(IP/端口/客户端)"
          onclick={() => onShowPeers && onShowPeers()}
        >
          <CircleCheck size={13} />
          {#if announceFeedback.count > 0}
            汇报正常 · 第 {announceFeedback.count} 次 · 做种 {announceFeedback.seeders} / 下载
            {announceFeedback.leechers} ⓘ
          {:else}
            等待首次汇报...
          {/if}
        </button>
      {/if}
    </div>

    {#if startFaking && stopFaking}
      <div
        class={cn(
          embedded
            ? 'flex shrink-0 items-center gap-2'
            : 'flex w-full items-center gap-2 overflow-x-auto pb-1 lg:w-auto lg:justify-end lg:overflow-visible lg:pb-0'
        )}
      >
        {#if !isRunning}
          <button
            onclick={startFaking}
            aria-label="开始伪造"
            title="开始"
            class={cn(
              embedded
                ? `${embeddedActionBase} shrink-0 border-stat-upload/30 bg-stat-upload text-white shadow-sm hover:bg-stat-upload/90`
                : `${actionBase} shrink-0 whitespace-nowrap border-stat-upload/30 bg-stat-upload text-white hover:bg-stat-upload/90`
            )}
          >
            <Play size={13} fill="currentColor" />
            {#if !embedded}<span>开始</span>{/if}
          </button>
        {:else}
          {#if !isPaused}
            <button
              onclick={pauseFaking}
              aria-label="暂停伪造"
              title="暂停"
              class={cn(
                embedded
                  ? `${embeddedActionBase} shrink-0 border-stat-ratio/30 bg-stat-ratio/10 text-stat-ratio hover:bg-stat-ratio/15`
                  : `${actionBase} shrink-0 whitespace-nowrap border-stat-ratio/30 bg-stat-ratio/10 text-stat-ratio hover:bg-stat-ratio/15`
              )}
            >
              <Pause size={13} fill="currentColor" />
              {#if !embedded}<span>暂停</span>{/if}
            </button>
          {:else}
            <button
              onclick={resumeFaking}
              aria-label="恢复伪造"
              title="恢复"
              class={cn(
                embedded
                  ? `${embeddedActionBase} shrink-0 border-primary/30 bg-primary/10 text-primary hover:bg-primary/15`
                  : `${actionBase} shrink-0 whitespace-nowrap border-primary/30 bg-primary/10 text-primary hover:bg-primary/15`
              )}
            >
              <Play size={13} fill="currentColor" />
              {#if !embedded}<span>恢复</span>{/if}
            </button>
          {/if}
          <button
            onclick={manualUpdate}
            aria-label="更新统计"
            title="更新"
            class={cn(
              embedded
                ? `${embeddedActionBase} shrink-0 border-border/70 bg-background/65 text-muted-foreground hover:border-border hover:bg-secondary/60 hover:text-foreground`
                : `${actionBase} shrink-0 whitespace-nowrap border-border/70 bg-background/65 text-muted-foreground hover:border-border hover:bg-secondary/60 hover:text-foreground`
            )}
          >
            <RefreshCw size={13} />
            {#if !embedded}<span>更新</span>{/if}
          </button>
          <button
            onclick={stopFaking}
            aria-label="停止伪造"
            title="停止"
            class={cn(
              embedded
                ? `${embeddedActionBase} shrink-0 border-destructive/20 bg-destructive/10 text-destructive hover:bg-destructive/15`
                : `${actionBase} shrink-0 whitespace-nowrap border-destructive/20 bg-destructive/10 text-destructive hover:bg-destructive/15`
            )}
          >
            <Square size={13} fill="currentColor" />
            {#if !embedded}<span>停止</span>{/if}
          </button>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.2);
    }
  }

  .animate-pulse {
    animation: pulse 1.5s ease-in-out infinite;
  }
</style>
