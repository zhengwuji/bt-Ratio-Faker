export const GRID_STATE_FILTER_OPTIONS = [
  { value: 'all', label: '全部状态', icon: 'circle', tone: 'text-muted-foreground' },
  { value: 'starting', label: '启动中', icon: 'loader', tone: 'text-primary', spin: true },
  { value: 'running', label: 'Running', icon: 'circle', tone: 'text-stat-upload' },
  { value: 'stopping', label: '停止中', icon: 'loader', tone: 'text-stat-danger', spin: true },
  { value: 'paused', label: 'Paused', icon: 'pause', tone: 'text-stat-ratio' },
  { value: 'idle', label: 'Idle', icon: 'moon', tone: 'text-violet-500' },
  { value: 'stopped', label: '已停止', icon: 'square', tone: 'text-muted-foreground' },
];

export function getGridStateFilterOption(value) {
  return (
    GRID_STATE_FILTER_OPTIONS.find(option => option.value === value) || GRID_STATE_FILTER_OPTIONS[0]
  );
}

export function getGridTagFilterOptions(tags = []) {
  const values = [...new Set(tags.filter(Boolean))].sort((a, b) => a.localeCompare(b));

  return [{ value: '', label: '全部标签' }, ...values.map(tag => ({ value: tag, label: tag }))];
}

export function getGridStateMeta(value) {
  return getGridStateFilterOption(value);
}
