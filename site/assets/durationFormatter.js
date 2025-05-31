const divMod = (n, m) => [Math.floor(n / m), n % m];

const createDurationFormatter = (locale, unitDisplay = 'long') => {
    const
        timeUnitFormatter = (locale, unit, unitDisplay) =>
            Intl.NumberFormat(locale, { style: 'unit', unit, unitDisplay }).format,
        fmtHours = timeUnitFormatter(locale, 'hour', unitDisplay),
        fmtMinutes = timeUnitFormatter(locale, 'minute', unitDisplay),
        fmtList = new Intl.ListFormat(locale, { style: 'long', type: 'conjunction' });
    return (minutes) => {
        const [hrs, mins] = divMod(minutes, 60);
        return fmtList.format([
            hrs ? fmtHours(hrs) : null,
            mins ? fmtMinutes(mins) : null
        ].filter(v => v !== null));
    }
};