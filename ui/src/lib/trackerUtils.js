export function normalizeTrackerHost(value) {
  if (!value) return '';
  return value.trim().replace(/\.+$/, '').toLowerCase();
}

export function extractTrackerHost(value) {
  if (!value) return '';

  const trimmed = value.trim();
  if (!trimmed) return '';

  try {
    const parsed = new URL(trimmed);
    return normalizeTrackerHost(parsed.hostname);
  } catch {
    // ignore and try fallback parsing
  }

  try {
    const parsed = new URL(`https://${trimmed}`);
    return normalizeTrackerHost(parsed.hostname);
  } catch {
    const withoutScheme = trimmed.replace(/^[a-z]+:\/\//i, '');
    const host = withoutScheme.split('/')[0]?.split(':')[0] || '';
    return normalizeTrackerHost(host);
  }
}

export function getPrimaryTrackerHost(instance) {
  const explicit = normalizeTrackerHost(
    instance?.primaryTrackerHost || instance?.primary_tracker_host
  );
  if (explicit) {
    return explicit;
  }

  return extractTrackerHost(instance?.torrent?.announce || '');
}

export function buildTrackerFilterEntries(instances, filters = {}) {
  const counts = new Map();
  const trackerSearch = normalizeTrackerHost(filters.trackerSearch || '');

  for (const instance of instances || []) {
    const host = getPrimaryTrackerHost(instance);
    if (!host) continue;

    if (trackerSearch && !host.includes(trackerSearch)) {
      continue;
    }

    counts.set(host, (counts.get(host) || 0) + 1);
  }

  return [...counts.entries()]
    .map(([host, count]) => ({
      value: host,
      label: host,
      count,
      initial: host.charAt(0).toUpperCase(),
      iconUrl: `https://${host}/favicon.ico`,
    }))
    .sort((left, right) => {
      if (right.count !== left.count) {
        return right.count - left.count;
      }
      return left.label.localeCompare(right.label);
    });
}

// 常见 tracker 子域前缀:推导主站时剥掉(仅当剩余部分仍是域名时)
const TRACKER_HOST_PREFIXES = ['tracker.', 'tracker1.', 'tracker2.', 't.', 'bt.', 'tr.', 'announce.', 'tk.'];

// 从 tracker announce 地址推导种子所属网站的主页 URL(用于点击打开并登录)
// udp://t.m-team.cc:6969/announce -> https://m-team.cc
// https://tracker.example.org/announce.php?passkey=.. -> https://example.org
// 推导不出(纯 IP / 无点主机名)时返回空串
export function getTrackerSiteUrl(value) {
  const trimmed = (value || '').trim();
  if (!trimmed) return '';

  let host = '';
  try {
    host = new URL(trimmed).hostname;
  } catch {
    const withoutScheme = trimmed.replace(/^[a-z]+:\/\//i, '');
    host = withoutScheme.split('/')[0]?.split(':')[0] || '';
  }
  host = normalizeTrackerHost(host);
  if (!host || !host.includes('.')) return '';
  // 纯 IPv4 地址没有可打开的网站
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(host)) return '';

  for (const prefix of TRACKER_HOST_PREFIXES) {
    if (host.startsWith(prefix) && host.slice(prefix.length).includes('.')) {
      host = host.slice(prefix.length);
      break;
    }
  }
  return `https://${host}`;
}
