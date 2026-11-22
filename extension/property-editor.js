const WRITABLE = 2;
const MAX_PROPERTY_INPUT = 4096;

function parseValue(spec, input) {
    const type = spec.value_type?.name;
    if (typeof input !== 'string' || input.length > MAX_PROPERTY_INPUT)
        throw new TypeError('Property value must be a string of at most 4096 characters.');
    if (type === 'gchararray') return input;
    if (type === 'gboolean') {
        if (input === 'true') return true;
        if (input === 'false') return false;
        throw new TypeError('Boolean values must be true or false.');
    }
    const numericTypes = {
        gint: Number.isSafeInteger, guint: Number.isSafeInteger,
        gint64: Number.isSafeInteger, guint64: Number.isSafeInteger,
        gfloat: Number.isFinite, gdouble: Number.isFinite,
    };
    const validate = numericTypes[type];
    if (!validate) throw new TypeError('Unsupported property type: ' + (type || 'unknown') + '.');
    if (input.trim() === '') throw new TypeError('Numeric property values cannot be empty.');
    const value = Number(input);
    if (!validate(value)) throw new TypeError('Property value is not a valid number for this type.');
    if ((type === 'guint' || type === 'guint64') && value < 0)
        throw new RangeError('Unsigned properties cannot be negative.');
    if (Number.isFinite(spec.minimum) && value < spec.minimum)
        throw new RangeError('Property value must be at least ' + spec.minimum + '.');
    if (Number.isFinite(spec.maximum) && value > spec.maximum)
        throw new RangeError('Property value must be at most ' + spec.maximum + '.');
    return value;
}

export function setActorProperty(actor, name, input) {
    if (typeof name !== 'string' || name.length === 0 || name.length > 256)
        return { ok: false, error: 'Property name must contain 1 to 256 characters.' };
    if (!actor || typeof actor.set_property !== 'function'
        || typeof actor.constructor?.list_properties !== 'function')
        return { ok: false, error: 'Actor does not support GObject properties.' };
    try {
        const spec = actor.constructor.list_properties().find(property => property.name === name);
        if (!spec) throw new Error('Property does not exist on this actor.');
        if ((spec.flags & WRITABLE) === 0) throw new Error('Property is read-only.');
        const value = parseValue(spec, input);
        actor.set_property(name, value);
        return { ok: true, name, value };
    } catch (error) {
        return { ok: false, error: error?.message || String(error) };
    }
}