export function parseProcessStatus(status) {
    if (typeof status !== 'string')
        return null;

    const readKilobytes = field => {
        const match = status.match(new RegExp('^' + field + ':\\s+(\\d+)\\s+kB$', 'm'));
        if (!match)
            return null;

        const bytes = Number(match[1]) * 1024;
        return Number.isSafeInteger(bytes) ? bytes : null;
    };
    const readCounter = field => {
        const line = status.split('\n').find(entry => entry.startsWith(field + ':'));
        const value = line?.slice(field.length + 1).trim();
        if (!value || !/^\d+$/.test(value))
            return null;

        const counter = Number(value);
        return Number.isSafeInteger(counter) ? counter : null;
    };

    return {
        residentBytes: readKilobytes('VmRSS'),
        peakResidentBytes: readKilobytes('VmHWM'),
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
    return Number.isSafeInteger(nanoseconds) ? nanoseconds : null;
}

export function calculateCpuPercent(cpuDeltaNs, elapsedUs) {
    if (!Number.isFinite(cpuDeltaNs) || !Number.isFinite(elapsedUs)
        || cpuDeltaNs < 0 || elapsedUs <= 0)
        return null;

    return Math.round(cpuDeltaNs / (elapsedUs * 1000) * 10000) / 100;
}
