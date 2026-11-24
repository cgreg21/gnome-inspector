import assert from 'node:assert/strict';
import test from 'node:test';
import { calculateCpuPercent, parseProcessCpuTime, parseProcessStatus } from './process-metrics.js';

test('parses process memory, thread and context-switch counters', () => {
    const result = parseProcessStatus([
        'Name:\tgnome-shell',
        'VmHWM:\t320000 kB',
        'VmRSS:\t280000 kB',
        'Threads:\t14',
        'voluntary_ctxt_switches:\t1200',
        'nonvoluntary_ctxt_switches:\t35',
    ].join('\n'));

    assert.deepEqual(result, {
        residentBytes: 280000 * 1024,
        peakResidentBytes: 320000 * 1024,
        threadCount: 14,
        voluntaryContextSwitches: 1200,
        involuntaryContextSwitches: 35,
    });
});

test('keeps unavailable process status counters null', () => {
    assert.deepEqual(parseProcessStatus('Name:\tgnome-shell'), {
        residentBytes: null,
        peakResidentBytes: null,
        threadCount: null,
        voluntaryContextSwitches: null,
        involuntaryContextSwitches: null,
    });
    assert.equal(parseProcessStatus(null), null);
});

test('parses scheduler runtime in nanoseconds', () => {
    assert.equal(parseProcessCpuTime('123456789 42 7'), 123456789);
});

test('returns null for unavailable or invalid scheduler runtime', () => {
    assert.equal(parseProcessCpuTime(''), null);
    assert.equal(parseProcessCpuTime('not-a-number 42 7'), null);
    assert.equal(parseProcessCpuTime(null), null);
});

test('rejects process metrics outside the safe integer range', () => {
    const result = parseProcessStatus([
        'VmRSS: 9007199254740992 kB',
        'VmHWM: 320000 kB',
        'Threads: 9007199254740992',
        'voluntary_ctxt_switches: 9007199254740992',
        'nonvoluntary_ctxt_switches: 35',
    ].join('\\n'));

    assert.equal(result.residentBytes, null);
    assert.equal(result.peakResidentBytes, 320000 * 1024);
    assert.equal(result.threadCount, null);
    assert.equal(result.voluntaryContextSwitches, null);
    assert.equal(result.involuntaryContextSwitches, 35);
    assert.equal(parseProcessCpuTime('9007199254740992 42 7'), null);
});

test('calculates main-thread CPU utilization from elapsed time', () => {
    assert.equal(calculateCpuPercent(500_000_000, 1_000_000), 50);
    assert.equal(calculateCpuPercent(125_000_000, 1_000_000), 12.5);
});

test('returns null when CPU utilization cannot be measured', () => {
    assert.equal(calculateCpuPercent(null, 1_000_000), null);
    assert.equal(calculateCpuPercent(-1, 1_000_000), null);
    assert.equal(calculateCpuPercent(100, 0), null);
    assert.equal(calculateCpuPercent(100, -1), null);
});
