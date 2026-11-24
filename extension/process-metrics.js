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
