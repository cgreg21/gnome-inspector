const READABLE = 1;
const WRITABLE = 2;
const MAX_PROPERTIES = 64;
const MAX_PROPERTY_STRING = 256;

function summarizePropertyValue(value) {
    if (value === null || value === undefined
        || typeof value === 'boolean' || typeof value === 'number')
        return value;
    if (typeof value === 'string')
        return value.length > MAX_PROPERTY_STRING
            ? `${value.slice(0, MAX_PROPERTY_STRING)}…`
            : value;
    if (typeof value === 'bigint' || typeof value === 'symbol')
        return String(value);
    if (typeof value === 'object') {
        const type = value.constructor?.name || 'Object';
        try {
            const name = typeof value.get_name === 'function'
                ? value.get_name()
                : null;
            return name ? `${type} (${name})` : type;
        } catch {
            return type;
        }
    }
    return String(value);
}

export function buildActorTree(root) {
    const budget = { count: 1, truncated: false, incomplete: false };
    const visited = new WeakSet();
    const errorMessage = error => {
        try {
            return typeof error?.message === 'string'
                ? error.message
                : String(error);
        } catch {
            return 'Unknown error';
        }
    };

    function visit(actor, depth, path) {
        if (!actor || typeof actor !== 'object') {
            budget.incomplete = true;
            return {
                path,
                name: 'Unavailable actor',
                type: 'Unknown',
                visible: false,
                reactive: false,
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                children: [],
                actorError: 'Child is not an actor object',
            };
        }

        if (visited.has(actor)) {
            budget.incomplete = true;
            return {
                path,
                name: 'Repeated actor',
                type: 'Unknown',
                visible: false,
                reactive: false,
                x: 0,
                y: 0,
                width: 0,
                height: 0,
                children: [],
                actorError: 'Actor was already visited',
            };
        }
        visited.add(actor);

        const propertyErrors = {};
        const read = (property, getter, fallback) => {
            try {
                return getter() ?? fallback;
            } catch (error) {
                budget.incomplete = true;
                propertyErrors[property] = errorMessage(error);
                return fallback;
            }
        };
        const readString = (property, getter, fallback) => {
            const value = read(property, getter, fallback);
            if (typeof value !== 'string') {
                budget.incomplete = true;
                propertyErrors[property] ||= 'Property is not a string';
                return fallback;
            }
            return value || fallback;
        };
        const readBoolean = (property, getter) => {
            const value = read(property, getter, false);
            if (typeof value !== 'boolean') {
                budget.incomplete = true;
                propertyErrors[property] ||= 'Property is not a boolean';
                return false;
            }
            return value;
        };
        const readNumber = (property, getter) => {
            const value = read(property, getter, 0);
            if (typeof value !== 'number' || !Number.isFinite(value)) {
                budget.incomplete = true;
                propertyErrors[property] ||= 'Property is not a finite number';
                return 0;
            }
            return Math.round(value);
        };

        const type = readString('type', () => actor.constructor.name, 'Actor');
        const node = {
            path,
            name: readString('name', () => actor.get_name(), type),
            type,
            visible: readBoolean('visible', () => actor.visible),
            reactive: readBoolean('reactive', () => actor.reactive),
            x: readNumber('x', () => actor.x),
            y: readNumber('y', () => actor.y),
            width: readNumber('width', () => actor.width),
            height: readNumber('height', () => actor.height),
            children: [],
        };

        try {
            const specs = actor.constructor.list_properties();
            node.properties = [];
            node.propertyCount = specs.length;
            node.propertiesTruncated = specs.length > MAX_PROPERTIES;
            for (const spec of specs.slice(0, MAX_PROPERTIES)) {
                const readable = (spec.flags & READABLE) !== 0;
                const writable = (spec.flags & WRITABLE) !== 0;
                const property = {
                    name: spec.name,
                    type: spec.value_type?.name || 'Unknown',
                    readable,
                    writable,
                };
                if (readable) {
                    try {
                        property.value = summarizePropertyValue(
                            actor.get_property(spec.name)
                        );
                    } catch (error) {
                        budget.incomplete = true;
                        property.error = errorMessage(error);
                    }
                }
                node.properties.push(property);
            }
        } catch (error) {
            budget.incomplete = true;
            node.propertiesError = errorMessage(error);
        }

        if (Object.keys(propertyErrors).length > 0)
            node.propertyErrors = propertyErrors;

        let children;
        try {
            children = actor.get_children();
            if (!Array.isArray(children))
                throw new TypeError('get_children() did not return an array');
        } catch (error) {
            budget.incomplete = true;
            node.childrenError = errorMessage(error);
            return node;
        }

        if (depth >= 12) {
            budget.truncated ||= children.length > 0;
            return node;
        }

        for (let index = 0; index < children.length; index++) {
            const child = children[index];
            if (budget.count >= 1000) {
                budget.truncated = true;
                break;
            }

            budget.count++;
            const childPath = path === '' ? String(index) : `${path}/${index}`;
            node.children.push(visit(child, depth + 1, childPath));
        }

        return node;
    }

    const tree = visit(root, 0, '');
    return {
        actorCount: budget.count,
        truncated: budget.truncated,
        incomplete: budget.incomplete,
        root: tree,
    };
}
