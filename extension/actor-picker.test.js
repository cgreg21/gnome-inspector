import assert from 'node:assert/strict';
import test from 'node:test';
import { findActorPath } from './actor-picker.js';

class FakeActor {
    constructor(children = []) {
        this.children = children;
    }

    get_children() {
        return this.children;
    }
}

test('finds an actor using its stable child-index path', () => {
    const target = new FakeActor();
    const root = new FakeActor([new FakeActor(), new FakeActor([target])]);

    assert.equal(findActorPath(root, root), '');
    assert.equal(findActorPath(root, target), '1/0');
});

test('returns null when the actor is outside the stage tree', () => {
    assert.equal(findActorPath(new FakeActor(), new FakeActor()), null);
});

test('stops at the configured maximum depth', () => {
    const target = new FakeActor();
    const root = new FakeActor([new FakeActor([target])]);

    assert.equal(findActorPath(root, target, 1), null);
});

test('handles actors whose children cannot be read', () => {
    const broken = new FakeActor();
    broken.get_children = () => {
        throw new Error('unavailable');
    };

    assert.equal(findActorPath(broken, new FakeActor()), null);
});
