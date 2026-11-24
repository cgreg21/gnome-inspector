export function parseProcessStatus(status) {
    if (typeof status !== 'string')
        return null;

    const resident = status.match(/^VmRSS:\s+(\d+)\s+kB$/m);
    const peak = status.match(/^VmHWM:\s+(\d+)\s+kB$/m);
    const readCounter = field => {
        const line = status.split('\n').find(entry => entry.startsWith(`${field}:`));
        const value = line?.slice(field.length + 1).trim();
        return value && /^\d+$/.test(value) ? Number(value) : null;
    };

    return {
        residentBytes: resident ? Number(resident[1]) * 1024 : null,
        peakResidentBytes: peak ? Number(peak[1]) * 1024 : null,
        threadCount: readCounter('Threads'),
        voluntaryContextSwitches: readCounter('voluntary_ctxt_switches'),
        involuntaryContextSwitches: readCounter('nonvoluntary_ctxt_switches'),
    };
}

export function parseProcessCpuTime(schedstat) {
    if (typeof schedstat !== 'string')
        return null;

    const runtime = schedstat.trim().split(/\s+/, 1)[0];
    if (!/^\d+$/.test(runtime))
        return null;

    const nanoseconds = Number(runtime);
    return Number.isFinite(nanoseconds) ? nanoseconds : null;
}

export function calculateCpuPercent(cpuDeltaNs, elapsedUs) {
    if (!Number.isFinite(cpuDeltaNs) || !Number.isFinite(elapsedUs)
        || cpuDeltaNs < 0 || elapsedUs <= 0)
        return null;

    return Math.round(cpuDeltaNs / (elapsedUs * 1000) * 10000) / 100;
}
