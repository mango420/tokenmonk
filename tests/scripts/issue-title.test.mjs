import { test } from 'node:test';
import assert from 'node:assert/strict';
import { normalizeTitle, stripKey } from '../../scripts/issue-title.mjs';

test('stripKey removes the key in every separator form', () => {
  assert.equal(stripKey('TKM-3: foo'), 'foo');
  assert.equal(stripKey('TKM-3 : foo'), 'foo');
  assert.equal(stripKey('TKM-3 foo'), 'foo');
  assert.equal(stripKey('TKM-3:foo'), 'foo');
  assert.equal(stripKey('tkm-3:foo'), 'foo');
  assert.equal(stripKey('  TKM-3:   foo  '), 'foo');
});

test('stripKey leaves titles without a leading key untouched', () => {
  assert.equal(stripKey('foo'), 'foo');
  assert.equal(stripKey('foo TKM-3'), 'foo TKM-3');
  assert.equal(stripKey('TKM-x: foo'), 'TKM-x: foo');
  assert.equal(stripKey('TKM3: foo'), 'TKM3: foo');
});

test('stripKey strips only one leading key', () => {
  assert.equal(stripKey('TKM-3: TKM-4: foo'), 'TKM-4: foo');
});

test('stripKey on a bare key yields an empty string', () => {
  assert.equal(stripKey('TKM-3'), '');
  assert.equal(stripKey('TKM-3:'), '');
  assert.equal(stripKey('   '), '');
});

test('normalizeTitle formats the canonical form', () => {
  assert.equal(normalizeTitle('foo', 3), 'TKM-3: foo');
  assert.equal(normalizeTitle('TKM-3: foo', 3), 'TKM-3: foo');
});

test('normalizeTitle repairs separator variants without doubling the key', () => {
  assert.equal(normalizeTitle('TKM-3 : foo', 3), 'TKM-3: foo');
  assert.equal(normalizeTitle('TKM-3 foo', 3), 'TKM-3: foo');
  assert.equal(normalizeTitle('tkm-3:foo', 3), 'TKM-3: foo');
});

test('normalizeTitle replaces a stale or wrong key', () => {
  assert.equal(normalizeTitle('TKM-99: foo', 3), 'TKM-3: foo');
  assert.equal(normalizeTitle('tkm-99 foo', 3), 'TKM-3: foo');
});

test('normalizeTitle is idempotent', () => {
  const once = normalizeTitle('tkm-7 : do the thing', 12);
  assert.equal(normalizeTitle(once, 12), once);
});

test('normalizeTitle with an empty rest returns the bare key', () => {
  assert.equal(normalizeTitle('', 5), 'TKM-5');
  assert.equal(normalizeTitle('TKM-1:', 5), 'TKM-5');
});

test('normalizeTitle keeps the rest of the title as is', () => {
  assert.equal(normalizeTitle('fix(ui): a  b', 8), 'TKM-8: fix(ui): a  b');
});
