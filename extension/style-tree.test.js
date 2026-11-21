import assert from 'node:assert/strict';
import test from 'node:test';
import { buildStyleTree, resolveActorPath } from './style-tree.js';

class FakeWidget {
    constructor(name, children = []) {
        this.name = name;
        this.children = children;
    }

    get_name() { return this.name; }
    get_children() { return this.children; }
    get_style_class_name() { return 'button primary'; }
    get_style_pseudo_class() { return 'hover'; }
    get_inline_style() { return 'color: red;'; }
}

test('captures actor CSS classes, pseudo classes and inline styles', () => {
    const child = new FakeWidget('button');
    const result = buildStyleTree(new FakeWidget('stage', [child]));

    assert.equal(result.actorCount, 2);
    assert.equal(result.root.name, 'stage');
    assert.equal(result.root.children[0].styleClass, 'button primary');
    assert.equal(result.root.children[0].pseudoClass, 'hover');
    assert.equal(result.root.children[0].inlineStyle, 'color: red;');
});

test('limits style traversal and marks repeated actors only once', () => {
    const root = new FakeWidget('root');
    const child = new FakeWidget('child', [root]);
    root.children = [child];

    const result = buildStyleTree(root);

    assert.equal(result.actorCount, 2);
    assert.equal(result.root.children.length, 1);
    assert.equal(result.root.children[0].children.length, 0);
});

test('returns a useful partial snapshot when child enumeration fails', () => {
    const root = new FakeWidget('root');
    root.get_children = () => { throw new Error('unavailable'); };

    const result = buildStyleTree(root);

    assert.equal(result.root.name, 'root');
    assert.equal(result.root.childrenError, 'Unable to read child actors');
});

test('assigns stable child-index paths and resolves them', () => {
    const button = new FakeWidget('button');
    const panel = new FakeWidget('panel', [button]);
    const root = new FakeWidget('stage', [panel]);
    const snapshot = buildStyleTree(root);

    assert.equal(snapshot.root.path, '');
    assert.equal(snapshot.root.children[0].path, '0');
    assert.equal(snapshot.root.children[0].children[0].path, '0/0');
    assert.equal(resolveActorPath(root, '0/0'), button);
});

test('rejects malformed or out-of-range actor paths', () => {
    const root = new FakeWidget('stage', []);
    assert.equal(resolveActorPath(root, '../0'), null);
    assert.equal(resolveActorPath(root, '0'), null);
    assert.equal(resolveActorPath(root, '0/0/0/0/0/0/0/0/0/0/0/0/0'), null);
});
