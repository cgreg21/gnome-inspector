export function buildActorTree(root) {
    const budget = { count: 1, truncated: false };

    function visit(actor, depth) {
        const children = actor.get_children();
        const node = {
            name: actor.get_name() || actor.constructor.name || 'Actor',
            type: actor.constructor.name || 'Actor',
            visible: actor.visible,
            reactive: actor.reactive,
            x: Math.round(actor.x),
            y: Math.round(actor.y),
            width: Math.round(actor.width),
            height: Math.round(actor.height),
            children: [],
        };

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
        root: tree,
    };
}
