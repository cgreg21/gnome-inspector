import assert from 'node:assert/strict';
import test from 'node:test';
import { setActorProperty } from './property-editor.js';

class FakeActor {
    static list_properties() {
        return [
            { name: 'label', value_type: { name: 'gchararray' }, flags: 3 },
            { name: 'visible', value_type: { name: 'gboolean' }, flags: 3 },
            { name: 'opacity', value_type: { name: 'guint' }, flags: 3, minimum: 0, maximum: 255 },
            { name: 'scale', value_type: { name: 'gdouble' }, flags: 3, minimum: 0, maximum: 10 },
            { name: 'read-only', value_type: { name: 'gint' }, flags: 1 },
            { name: 'complex', value_type: { name: 'GObject' }, flags: 3 },
        ];
    }
    set_property(name, value) { this[name] = value; }
}

test('sets writable string, boolean and numeric properties', () => {
    const actor = new FakeActor();
    assert.deepEqual(setActorProperty(actor, 'label', 'Inspector'), { ok: true, name: 'label', value: 'Inspector' });
    assert.deepEqual(setActorProperty(actor, 'visible', 'false'), { ok: true, name: 'visible', value: false });
    assert.deepEqual(setActorProperty(actor, 'opacity', '180'), { ok: true, name: 'opacity', value: 180 });
    assert.deepEqual(setActorProperty(actor, 'scale', '1.5'), { ok: true, name: 'scale', value: 1.5 });
    assert.equal(actor.label, 'Inspector');
    assert.equal(actor.visible, false);
    assert.equal(actor.opacity, 180);
    assert.equal(actor.scale, 1.5);
});

test('rejects missing, read-only and unsupported properties', () => {
    const actor = new FakeActor();
    assert.equal(setActorProperty(actor, 'missing', '1').ok, false);
    assert.match(setActorProperty(actor, 'read-only', '1').error, /read-only/);
    assert.match(setActorProperty(actor, 'complex', 'x').error, /Unsupported/);
});

test('validates boolean, numeric syntax and numeric bounds', () => {
    const actor = new FakeActor();
    assert.equal(setActorProperty(actor, 'visible', 'yes').ok, false);
    assert.equal(setActorProperty(actor, 'opacity', '-1').ok, false);
    assert.equal(setActorProperty(actor, 'opacity', '256').ok, false);
    assert.equal(setActorProperty(actor, 'opacity', '1.5').ok, false);
    assert.equal(setActorProperty(actor, 'scale', '').ok, false);
    assert.equal(setActorProperty(actor, 'scale', 'NaN').ok, false);
});

test('does not call the setter when validation fails', () => {
    const actor = new FakeActor();
    const result = setActorProperty(actor, 'read-only', '3');
    assert.equal(result.ok, false);
    assert.equal(actor['read-only'], undefined);
});