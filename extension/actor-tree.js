export function buildActorTree(root) {
    const budget = { count: 1, truncated: false, incomplete: false };

    function visit(actor, depth) {
        const propertyErrors = {};
        const read = (property, getter, fallback) => {
            try {
                return getter() ?? fallback;
            } catch (error) {
                budget.incomplete = true;
                propertyErrors[property] = error.message || String(error);
                return fallback;
            }
        };

        const type = read('type', () => actor.constructor.name, 'Actor');
        const node = {
            name: read('name', () => actor.get_name(), null) || type,
            type,
            visible: read('visible', () => actor.visible, false),
            reactive: read('reactive', () => actor.reactive, false),
            x: Math.round(read('x', () => actor.x, 0)),
            y: Math.round(read('y', () => actor.y, 0)),
            width: Math.round(read('width', () => actor.width, 0)),
            height: Math.round(read('height', () => actor.height, 0)),
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
            node.childrenError = error.message || String(error);
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
