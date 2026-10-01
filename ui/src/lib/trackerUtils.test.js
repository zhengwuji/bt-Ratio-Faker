import assert from 'node:assert/strict';
import test from 'node:test';

import {
  buildTrackerFilterEntries,
  extractTrackerHost,
  getPrimaryTrackerHost,
  getTrackerSiteUrl,
  normalizeTrackerHost,
} from './trackerUtils.js';

test('extractTrackerHost normalizes announce urls and host-like values', () => {
  assert.equal(
    extractTrackerHost('https://Tracker.Example.com:443/announce?passkey=123'),
    'tracker.example.com'
  );
  assert.equal(extractTrackerHost('udp://open.stealth.si:80/announce'), 'open.stealth.si');
  assert.equal(extractTrackerHost('tracker.torrent.eu.org/announce'), 'tracker.torrent.eu.org');
  assert.equal(extractTrackerHost(''), '');
});

test('getPrimaryTrackerHost prefers summary field and falls back safely', () => {
  assert.equal(getPrimaryTrackerHost({ primaryTrackerHost: 'A.B.C' }), 'a.b.c');
  assert.equal(getPrimaryTrackerHost({ primary_tracker_host: 'X.Y.Z' }), 'x.y.z');
  assert.equal(
    getPrimaryTrackerHost({ torrent: { announce: 'https://c411.org/announce' } }),
    'c411.org'
  );
});

test('buildTrackerFilterEntries aggregates counts and applies tracker search', () => {
  const entries = buildTrackerFilterEntries(
    [
      { primaryTrackerHost: 'c411.org' },
      { primaryTrackerHost: 'c411.org' },
      { primaryTrackerHost: 'nyaa.tracker.wf' },
      { primaryTrackerHost: '' },
    ],
    { trackerSearch: 'c4' }
  );

  assert.deepEqual(entries, [
    {
      value: 'c411.org',
      label: 'c411.org',
      count: 2,
      initial: 'C',
      iconUrl: 'https://c411.org/favicon.ico',
    },
  ]);
  assert.equal(normalizeTrackerHost('Test.Host.'), 'test.host');
});

test('multi-select tracker filtering can match more than one host', () => {
  const selected = new Set(['c411.org', 'nyaa.tracker.wf']);
  const values = [
    { primaryTrackerHost: 'c411.org' },
    { primaryTrackerHost: 'open.stealth.si' },
    { primaryTrackerHost: 'nyaa.tracker.wf' },
  ]
    .filter(instance => selected.has(getPrimaryTrackerHost(instance)))
    .map(instance => getPrimaryTrackerHost(instance));

  assert.deepEqual(values, ['c411.org', 'nyaa.tracker.wf']);
});

test('getTrackerSiteUrl derives the main site from tracker announce urls', () => {
  assert.equal(getTrackerSiteUrl('udp://t.m-team.cc:6969/announce'), 'https://m-team.cc');
  assert.equal(getTrackerSiteUrl('https://tracker.example.org/announce.php?passkey=x'), 'https://example.org');
  assert.equal(getTrackerSiteUrl('http://tracker1.xyz.com:2710/announce'), 'https://xyz.com');
  assert.equal(getTrackerSiteUrl('udp://bt.example.com:80/announce'), 'https://example.com');
  assert.equal(getTrackerSiteUrl('https://tracker.example.org/announce'), 'https://example.org');
});

test('getTrackerSiteUrl returns empty for hosts without a website', () => {
  assert.equal(getTrackerSiteUrl('udp://127.0.0.1:6969/announce'), '');
  assert.equal(getTrackerSiteUrl('udp://192.168.1.10:6969/announce'), '');
  assert.equal(getTrackerSiteUrl(''), '');
  assert.equal(getTrackerSiteUrl(null), '');
});
