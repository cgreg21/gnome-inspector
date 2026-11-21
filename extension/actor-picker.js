export function findActorPath(root, target, maxDepth = 12) {
    if (!root || !target)
        return null;

    function visit(actor, path, depth) {
        if (actor === target)
            return path;
        if (depth >= maxDepth)
            return null;

        let children;
        try {
            children = actor.get_children();
        } catch {
            return null;
        }
        if (!Array.isArray(children))
            return null;

        for (let index = 0; index < children.length; index++) {
            const childPath = path === '' ? String(index) : `${path}/${index}`;
            const result = visit(children[index], childPath, depth + 1);
            if (result !== null)
                return result;
        }
        return null;
    }

    return visit(root, '', 0);
}
