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

export function parseProcessCpuTime(schedstat) {
    if (typeof schedstat !== 'string')
        return null;

    const runtime = schedstat.trim().split(/\s+/, 1)[0];
    if (!/^\d+$/.test(runtime))
        return null;

    const nanoseconds = Number(runtime);
    return Number.isFinite(nanoseconds) ? nanoseconds : null;
}
