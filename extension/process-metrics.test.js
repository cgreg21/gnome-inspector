import assert from 'node:assert/strict';
import test from 'node:test';
import { parseProcessMemory } from './process-metrics.js';

test('parses resident and peak process memory from proc status', () => {
    const result = parseProcessMemory([
        'Name:\tgnome-shell',
        'VmPeak:\t900000 kB',
        'VmHWM:\t320000 kB',
        'VmRSS:\t280000 kB',
        'Threads:\t12',
    ].join('\n'));

    assert.deepEqual(result, {
        residentBytes: 280000 * 1024,
        peakResidentBytes: 320000 * 1024,
    });
});

test('returns null when process memory fields are unavailable', () => {
    assert.equal(parseProcessMemory('Name:\tgnome-shell\nVmRSS:\t12 kB'), null);
    assert.equal(parseProcessMemory(null), null);
});
