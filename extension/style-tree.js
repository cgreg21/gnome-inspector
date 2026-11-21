export function buildStyleTree(root) {
    const budget = { count: 0, truncated: false };
    const visited = new WeakSet();

    function read(actor, method) {
        try {
            const value = actor[method]?.();
            return typeof value === 'string' ? value : '';
        } catch {
            return '';
        }
    }

    function visit(actor, depth, path) {
        if (!actor || typeof actor !== 'object' || visited.has(actor))
            return null;

        visited.add(actor);
        budget.count++;

        const node = {
            name: read(actor, 'get_name') || actor.constructor?.name || 'Actor',
            type: actor.constructor?.name || 'Actor',
            path,
            styleClass: read(actor, 'get_style_class_name'),
            pseudoClass: read(actor, 'get_style_pseudo_class'),
            inlineStyle: read(actor, 'get_inline_style'),
            children: [],
        };

        if (depth >= 12) {
            try {
                budget.truncated ||= actor.get_children().length > 0;
            } catch {
                node.childrenError = 'Unable to read child actors';
            }
            return node;
        }

        let children;
        try {
            children = actor.get_children();
            if (!Array.isArray(children))
                throw new TypeError('Invalid child list');
        } catch {
            node.childrenError = 'Unable to read child actors';
            return node;
        }

        for (let index = 0; index < children.length; index++) {
            const child = children[index];
            if (budget.count >= 1000) {
                budget.truncated = true;
                break;
            }
            const childPath = path === '' ? String(index) : `${path}/${index}`;
            const childNode = visit(child, depth + 1, childPath);
            if (childNode)
                node.children.push(childNode);
        }

        return node;
    }

    const tree = visit(root, 0, '');
    return { actorCount: budget.count, truncated: budget.truncated, root: tree };
}

export function resolveActorPath(root, path) {
    if (typeof path !== 'string' || path.length > 64)
        return null;
    if (path === '')
        return root;

    const indexes = path.split('/');
    if (indexes.length > 12 || indexes.some(index => !/^(0|[1-9][0-9]*)$/.test(index)))
        return null;

    let actor = root;
    for (const index of indexes) {
        let children;
        try {
            children = actor?.get_children?.();
        } catch {
            return null;
        }
        const childIndex = Number(index);
        if (!Array.isArray(children) || childIndex >= children.length)
            return null;
        actor = children[childIndex];
    }

    return actor ?? null;
}
