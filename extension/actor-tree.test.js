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
