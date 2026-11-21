import assert from 'node:assert/strict';
import test from 'node:test';
import { buildActorTree } from './actor-tree.js';

class FakeActor {
    constructor(name, children = []) {
        this._name = name;
        this._children = children;
        this.visible = true;
        this.reactive = false;
        this.x = 1;
        this.y = 2;
        this.width = 30;
        this.height = 40;
    }

    get_name() {
        return this._name;
    }

    get_children() {
        return this._children;
    }
}

test('builds a read-only actor tree with geometry and visibility', () => {
    const child = new FakeActor('child');
    const root = new FakeActor('root', [child]);

    const result = buildActorTree(root);

    assert.equal(result.actorCount, 2);
    assert.equal(result.truncated, false);
    assert.equal(result.root.name, 'root');
    assert.equal(result.root.children[0].name, 'child');
    assert.equal(result.root.children[0].width, 30);
    assert.equal(result.root.children[0].visible, true);
});

test('caps snapshots at 1,000 actors', () => {
    const children = Array.from(
        { length: 1_001 },
        (_, index) => new FakeActor(`child-${index}`)
    );
    const result = buildActorTree(new FakeActor('root', children));

    assert.equal(result.actorCount, 1_000);
    assert.equal(result.root.children.length, 999);
    assert.equal(result.truncated, true);
});

test('caps traversal depth at 12 levels', () => {
    let root = new FakeActor('leaf');
    for (let depth = 0; depth < 15; depth++)
        root = new FakeActor(`level-${depth}`, [root]);

    const result = buildActorTree(root);

    assert.equal(result.actorCount, 13);
    assert.equal(result.truncated, true);
});

test('marks the snapshot incomplete when a child list cannot be read', () => {
    const broken = new FakeActor('broken');
    broken.get_children = () => {
        throw new Error('children unavailable');
    };

    const result = buildActorTree(new FakeActor('root', [broken]));

    assert.equal(result.actorCount, 2);
    assert.equal(result.incomplete, true);
    assert.equal(result.truncated, false);
    assert.equal(result.root.children[0].childrenError, 'children unavailable');
    assert.deepEqual(result.root.children[0].children, []);
});

test('keeps the snapshot when an actor property getter throws', () => {
    const broken = new FakeActor('broken');
    Object.defineProperty(broken, 'width', {
        get() {
            throw new Error('width unavailable');
        },
    });
    broken.get_name = () => {
        throw new Error('name unavailable');
    };

    const result = buildActorTree(new FakeActor('root', [broken]));
    const node = result.root.children[0];

    assert.equal(result.incomplete, true);
    assert.equal(node.name, 'FakeActor');
    assert.equal(node.width, 0);
    assert.deepEqual(node.propertyErrors, {
        name: 'name unavailable',
        width: 'width unavailable',
    });
});

test('rejects an invalid child list without aborting the snapshot', () => {
    const broken = new FakeActor('broken');
    broken.get_children = () => null;

    const result = buildActorTree(new FakeActor('root', [broken]));

    assert.equal(result.incomplete, true);
    assert.equal(result.root.children[0].childrenError,
        'get_children() did not return an array');
});

test('uses safe geometry values for non-finite and non-numeric properties', () => {
    const broken = new FakeActor('broken');
    broken.x = Number.NaN;
    broken.y = '12';
    broken.width = Number.POSITIVE_INFINITY;

    const result = buildActorTree(new FakeActor('root', [broken]));
    const node = result.root.children[0];

    assert.equal(result.incomplete, true);
    assert.equal(node.x, 0);
    assert.equal(node.y, 0);
    assert.equal(node.width, 0);
    assert.deepEqual(node.propertyErrors, {
        x: 'Property is not a finite number',
        y: 'Property is not a finite number',
        width: 'Property is not a finite number',
    });
});

test('keeps an invalid child entry as an explicit incomplete node', () => {
    const result = buildActorTree(new FakeActor('root', [null]));

    assert.equal(result.actorCount, 2);
    assert.equal(result.incomplete, true);
    assert.equal(result.root.children[0].actorError,
        'Child is not an actor object');
});

test('stops cyclic actor references without recursing forever', () => {
    const root = new FakeActor('root');
    const child = new FakeActor('child');
    root._children = [child];
    child._children = [root];

    const result = buildActorTree(root);

    assert.equal(result.actorCount, 3);
    assert.equal(result.incomplete, true);
    assert.equal(result.root.children[0].children[0].actorError,
        'Actor was already visited');
});

test('uses safe values for invalid names, types, and boolean properties', () => {
    const broken = new FakeActor('broken');
    broken.get_name = () => Symbol('invalid name');
    broken.visible = 'yes';
    broken.reactive = 1;
    Object.defineProperty(broken, 'constructor', {
        get() {
            return { name: 42 };
        },
    });

    const result = buildActorTree(new FakeActor('root', [broken]));
    const node = result.root.children[0];

    assert.equal(result.incomplete, true);
    assert.equal(node.name, 'Actor');
    assert.equal(node.type, 'Actor');
    assert.equal(node.visible, false);
    assert.equal(node.reactive, false);
    assert.deepEqual(node.propertyErrors, {
        type: 'Property is not a string',
        name: 'Property is not a string',
        visible: 'Property is not a boolean',
        reactive: 'Property is not a boolean',
    });
});

test('formats unusual thrown values without aborting the snapshot', () => {
    const broken = new FakeActor('broken');
    Object.defineProperty(broken, 'width', {
        get() {
            throw null;
        },
    });
    broken.get_children = () => {
        throw {
            get message() {
                throw new Error('message unavailable');
            },
        };
    };

    const result = buildActorTree(new FakeActor('root', [broken]));
    const node = result.root.children[0];

    assert.equal(result.incomplete, true);
    assert.equal(node.width, 0);
    assert.equal(node.propertyErrors.width, 'null');
    assert.equal(node.childrenError, 'Unknown error');
});

test('assigns stable child-index paths to every actor', () => {
    const target = new FakeActor('target');
    const root = new FakeActor('root', [new FakeActor('sibling'), target]);
    const result = buildActorTree(root);

    assert.equal(result.root.path, '');
    assert.equal(result.root.children[0].path, '0');
    assert.equal(result.root.children[1].path, '1');
});
