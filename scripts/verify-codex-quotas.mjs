import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { daemonExecutable, connect } from './codex-probe-client.mjs';

const root = resolve(import.meta.dirname, '..');
const dir = join(root, 'scratch', `codex-quota-proof-${randomUUID()}`);
mkdirSync(dir, { recursive:true });
const file = join(dir, 'quotas.json');
const capy = process.env.CAPY_TEST_EXE || join(root, 'src-tauri/target/release/capy.exe');
execFileSync(capy, ['--quota-report', file], {windowsHide:true, timeout:15000});
const rows = JSON.parse(readFileSync(file, 'utf8'));
const client = await connect(daemonExecutable());
try {
  const before = await client.request('account/read', {refreshToken:false});
  const limits = await client.request('account/rateLimits/read', {});
  const after = await client.request('account/read', {refreshToken:false});
  assert.deepEqual(before.account, after.account, 'Account changed during independent read');
  assert.equal(before.account.type, 'chatgpt');
  assert.equal(typeof before.account.email, 'string');
  const buckets = limits.rateLimitsByLimitId ?? {[limits.rateLimits.limitId]:limits.rateLimits};
  const codex = rows.filter(r => r.provider === 'Codex');
  assert.ok(codex.length > 0, 'Missing real quota rows');
  for (const period of ['primary','secondary']) {
    const row = codex.find(r => r.period === period && r.state === 'fresh');
    assert.ok(row?.window, `No fresh ${period} evidence`);
    assert.equal(row.account, before.account.email, 'Quota account differs from source');
    const source = buckets[row.bucket]?.[period];
    assert.ok(source, 'Unknown source bucket/window');
    assert.equal(row.window.windowDurationMins, source.windowDurationMins);
    assert.equal(row.window.resetsAt, source.resetsAt);
    assert.ok(Number.isFinite(row.window.usedPercent) && row.window.usedPercent >= 0 && row.window.usedPercent <= 100);
    const age = Date.now() - row.observedAt;
    assert.ok(age >= 0 && age <= 120000);
    assert.ok(Date.now() < row.window.resetsAt * 1000);
  }
  for (const provider of ['Claude','Antigravity']) {
    const row = rows.find(r => r.provider === provider);
    assert.equal(row.state, 'unavailable');
    assert.equal(row.window, null);
  }
  console.log('PASS: real Codex primary/secondary quota source, stable account, reset units and absence handling');
} finally { await client.close(); }
