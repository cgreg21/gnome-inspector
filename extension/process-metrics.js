export function parseProcessMemory(status) {
    if (typeof status !== 'string')
        return null;

    const resident = status.match(/^VmRSS:\s+(\d+)\s+kB$/m);
    const peak = status.match(/^VmHWM:\s+(\d+)\s+kB$/m);
    if (!resident || !peak)
        return null;

    return {
        residentBytes: Number(resident[1]) * 1024,
        peakResidentBytes: Number(peak[1]) * 1024,
    };
}

export function parseProcessActivity(status) {
    if (typeof status !== 'string')
        return null;

    const readCounter = (name) => {
        const match = status.match(new RegExp(`^${name}:\\s+(\\d+)export function parseProcessMemory(status) {
    if (typeof status !== 'string')
        return null;

    const resident = status.match(/^VmRSS:\s+(\d+)\s+kB$/m);
    const peak = status.match(/^VmHWM:\s+(\d+)\s+kB$/m);
    if (!resident || !peak)
        return null;

    return {
        residentBytes: Number(resident[1]) * 1024,
        peakResidentBytes: Number(peak[1]) * 1024,
    };
}

, 'm'));
        return match ? Number(match[1]) : null;
    };

    return {
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
