import assert from 'node:assert/strict';
import test from 'node:test';

import {
  GRID_STATE_FILTER_OPTIONS,
  getGridStateFilterOption,
  getGridTagFilterOptions,
} from './gridFilterOptions.js';

test('getGridStateFilterOption falls back to all states', () => {
  assert.equal(getGridStateFilterOption('running').label, 'Running');
  assert.equal(getGridStateFilterOption('unknown').value, GRID_STATE_FILTER_OPTIONS[0].value);
});

test('getGridTagFilterOptions sorts and deduplicates tags', () => {
  assert.deepEqual(getGridTagFilterOptions(['beta', 'alpha', 'beta']), [
    { value: '', label: '全部标签' },
    { value: 'alpha', label: 'alpha' },
    { value: 'beta', label: 'beta' },
  ]);
});
