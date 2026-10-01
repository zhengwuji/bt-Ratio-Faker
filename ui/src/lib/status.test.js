import assert from 'node:assert/strict';
import test from 'node:test';

import {
  formatRetrySeconds,
  getIdlingReasonText,
  getIdlingStatus,
  getStatusFromStats,
  getTrackerIssue,
} from './status.js';
import { getGridLivePeers, getGridLiveRate, isGridLiveState } from './gridMetrics.js';

test('getIdlingReasonText handles snake_case and camelCase reasons', () => {
  assert.equal(getIdlingReasonText('no_leechers'), '没有可用的下载者');
  assert.equal(getIdlingReasonText('noLeechers'), '没有可用的下载者');
  assert.equal(getIdlingReasonText('no_seeders'), '没有可用的做种者');
  assert.equal(getIdlingReasonText('stop_condition_met'), '停止条件已满足');
});

test('getIdlingStatus avoids misleading fallback message', () => {
  assert.equal(getIdlingStatus(undefined).statusMessage, '空闲中');
  assert.equal(getIdlingStatus('unknown_reason').statusMessage, '空闲中');
});

test('getStatusFromStats uses idling reason when available', () => {
  assert.deepEqual(getStatusFromStats({ is_idling: true, idling_reason: 'noLeechers' }), {
    statusMessage: '空闲中 - 没有可用的下载者',
    statusType: 'idling',
    statusIcon: 'moon',
  });
});

test('getTrackerIssue supports tracker errors from stats and summaries', () => {
  assert.deepEqual(getTrackerIssue({ tracker_error: 'Tracker unavailable' }), {
    statusMessage: 'Tracker 不可用',
    statusType: 'warning',
    statusIcon: null,
    issueLabel: 'Tracker 异常',
  });

  assert.deepEqual(getTrackerIssue({ trackerError: 'Tracker 上未找到该种子' }), {
    statusMessage: 'Tracker 上未找到该种子',
    statusType: 'warning',
    statusIcon: null,
    issueLabel: 'Tracker 异常',
  });
});

test('getTrackerIssue formats visible retry countdown for temporary tracker failures', () => {
  const retryAtMs = Date.now() + 12_000;

  assert.deepEqual(
    getTrackerIssue({ tracker_error: 'Tracker unavailable', tracker_retry_at_ms: retryAtMs }),
    {
      statusMessage: 'Tracker 不可用,12 秒后重试',
      statusType: 'warning',
      statusIcon: null,
      issueLabel: 'Tracker 异常',
    }
  );
});

test('formatRetrySeconds rounds up remaining retry countdown', () => {
  assert.equal(formatRetrySeconds(1_500, 0), 2);
  assert.equal(formatRetrySeconds(1_000, 0), 1);
  assert.equal(formatRetrySeconds(0, 0), 0);
});

test('isNetworkConfigured defaults to true when status is absent', () => {
  const isNetworkConfigured = status => status?.configured !== false;
  assert.equal(isNetworkConfigured(null), true);
  assert.equal(isNetworkConfigured(undefined), true);
});

test('isNetworkConfigured returns false when backend reports no VPN configured', () => {
  const isNetworkConfigured = status => status?.configured !== false;
  assert.equal(isNetworkConfigured({ configured: false }), false);
});

test('shouldShowNetworkStatus hides unconfigured VPN state', () => {
  const shouldShowNetworkStatus = status => status?.configured !== false;
  assert.equal(shouldShowNetworkStatus({ configured: false }), false);
  assert.equal(shouldShowNetworkStatus({ configured: true }), true);
});

test('getVpnPortSyncEnabled requires both configuration and server enablement', () => {
  const getVpnPortSyncEnabled = status =>
    status?.configured !== false && (status?.vpn_port_sync_enabled ?? true);
  assert.equal(getVpnPortSyncEnabled({ configured: false, vpn_port_sync_enabled: true }), false);
  assert.equal(getVpnPortSyncEnabled({ configured: true, vpn_port_sync_enabled: false }), false);
  assert.equal(getVpnPortSyncEnabled({ configured: true, vpn_port_sync_enabled: true }), true);
});

test('grid live metrics exclude paused and stopped states', () => {
  assert.equal(isGridLiveState('running'), true);
  assert.equal(isGridLiveState('idle'), true);
  assert.equal(isGridLiveState('paused'), false);
  assert.equal(isGridLiveState('stopped'), false);
  assert.equal(getGridLiveRate('paused', 42.5), 0);
  assert.equal(getGridLiveRate('stopped', 42.5), 0);
  assert.equal(getGridLiveRate('running', 42.5), 42.5);
  assert.deepEqual(getGridLivePeers('paused', 10, 20), { seeders: null, leechers: null });
  assert.deepEqual(getGridLivePeers('stopped', 10, 20), { seeders: null, leechers: null });
  assert.deepEqual(getGridLivePeers('idle', 10, 20), { seeders: 10, leechers: 20 });
});
