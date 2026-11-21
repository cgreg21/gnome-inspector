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

export function buildComputedStyle(actor) {
    let node;
    try {
        node = actor?.get_theme_node?.();
    } catch {
        return { available: false, error: 'Unable to read computed theme node' };
    }

    if (!node)
        return { available: false };

    function read(method) {
        try {
            const value = node[method]?.();
            return typeof value === 'number' && Number.isFinite(value) ? value : null;
        } catch {
            return null;
        }
    }

    function readText(method) {
        try {
            const value = node[method]?.();
            return typeof value === 'string' ? value : null;
        } catch {
            return null;
        }
    }

    function readList(method) {
        try {
            const value = node[method]?.();
            return Array.isArray(value) ? value.filter(item => typeof item === 'string') : [];
        } catch {
            return [];
        }
    }

    let font = null;
    try {
        font = node.get_font?.()?.to_string?.() ?? null;
    } catch {
        // Some actors may not expose a readable font description.
    }

    return {
        available: true,
        node: readText('to_string'),
        elementId: readText('get_element_id'),
        elementClasses: readList('get_element_classes'),
        pseudoClasses: readList('get_pseudo_classes'),
        font,
        width: read('get_width'),
        height: read('get_height'),
        minWidth: read('get_min_width'),
        maxWidth: read('get_max_width'),
        minHeight: read('get_min_height'),
        maxHeight: read('get_max_height'),
        horizontalPadding: read('get_horizontal_padding'),
        verticalPadding: read('get_vertical_padding'),
        letterSpacing: read('get_letter_spacing'),
    };
}
