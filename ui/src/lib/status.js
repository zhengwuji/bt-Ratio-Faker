export function getIdlingReasonText(reason) {
  if (reason === 'stop_condition_met' || reason === 'stopConditionMet') {
    return '停止条件已满足';
  }

  if (reason === 'no_leechers' || reason === 'noLeechers') {
    return '没有可用的下载者';
  }

  if (reason === 'no_seeders' || reason === 'noSeeders') {
    return '没有可用的做种者';
  }

  return null;
}

export function getReadyStatus(message = '准备开始伪造') {
  return {
    statusMessage: message,
    statusType: 'idle',
    statusIcon: null,
  };
}

export function getRunningStatus(message = '正在伪造比率...') {
  return {
    statusMessage: message,
    statusType: 'running',
    statusIcon: 'rocket',
  };
}

export function getPausedStatus(message = '已暂停') {
  return {
    statusMessage: message,
    statusType: 'paused',
    statusIcon: 'pause',
  };
}

export function getTrackerInvalidStatus(message = 'Tracker 上未找到该种子') {
  return {
    statusMessage: message,
    statusType: 'warning',
    statusIcon: null,
  };
}

export function formatRetrySeconds(retryAtMs, nowMs = Date.now()) {
  if (retryAtMs == null) {
    return null;
  }

  const remainingMs = Math.max(0, retryAtMs - nowMs);
  return Math.ceil(remainingMs / 1000);
}

// 后端哨兵错误串保持英文(用于重试判定),仅在展示层映射为中文
const TRACKER_MESSAGE_I18N = {
  'Tracker unavailable': 'Tracker 不可用',
  'Torrent not found on tracker': 'Tracker 上未找到该种子',
};

// 把 tracker 返回的英文拒绝原因翻译成可操作的中文提示
function translateTrackerMessage(message) {
  if (TRACKER_MESSAGE_I18N[message]) {
    return TRACKER_MESSAGE_I18N[message];
  }
  if (/blacklisted/i.test(message)) {
    return `Tracker 拒绝了当前端口(已列入黑名单):${message} — 请修改实例「端口」(例如 51413)后重新开始`;
  }
  if (/unregistered|not registered|not authorized|invalid credential|passkey|credential/i.test(message)) {
    return `Tracker 拒绝了该种子(认证/注册信息无效):${message} — 请确认种子是否仍有效或重新下载种子`;
  }
  return message;
}

export function getTrackerIssue(stats) {
  const message = stats?.tracker_error || stats?.trackerError;
  if (!message) {
    return null;
  }

  const retryAtMs = stats?.tracker_retry_at_ms ?? stats?.trackerRetryAtMs;
  const retrySecs = formatRetrySeconds(retryAtMs);
  const statusMessage =
    message === 'Tracker unavailable' && retrySecs != null
      ? retrySecs > 0
        ? `Tracker 不可用,${retrySecs} 秒后重试`
        : 'Tracker 不可用,正在重试'
      : translateTrackerMessage(message);

  return {
    statusMessage,
    statusType: 'warning',
    statusIcon: null,
    issueLabel: 'Tracker 异常',
  };
}

export function getIdlingStatus(reason) {
  const text = getIdlingReasonText(reason);

  return {
    statusMessage: text ? `空闲中 - ${text}` : '空闲中',
    statusType: 'idling',
    statusIcon: 'moon',
  };
}

export function getStatusFromStats(stats) {
  const trackerIssue = getTrackerIssue(stats);
  if (trackerIssue) {
    return trackerIssue;
  }

  if (stats?.is_idling) {
    return getIdlingStatus(stats.idling_reason);
  }

  return getRunningStatus();
}
