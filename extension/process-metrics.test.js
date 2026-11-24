import assert from 'node:assert/strict';
import test from 'node:test';
import { parseProcessCpuTime, parseProcessStatus } from './process-metrics.js';

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
