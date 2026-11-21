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

    function visit(actor, depth) {
        if (!actor || typeof actor !== 'object') {
            budget.incomplete = true;
            return {
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

        for (const child of children) {
            if (budget.count >= 1000) {
                budget.truncated = true;
                break;
            }

            budget.count++;
            node.children.push(visit(child, depth + 1));
        }

        return node;
    }

    const tree = visit(root, 0);
    return {
        actorCount: budget.count,
        truncated: budget.truncated,
        incomplete: budget.incomplete,
        root: tree,
    };
}
