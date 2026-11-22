import assert from 'node:assert/strict';
import test from 'node:test';
import { reloadCustomStylesheets } from './theme-stylesheets.js';

function stylesheet(path) {
    return { get_path: () => path };
}

test('reloads every custom stylesheet', () => {
    const calls = [];
    const files = [stylesheet('/tmp/one.css'), stylesheet('/tmp/two.css')];
    const theme = {
        get_custom_stylesheets: () => files,
        unload_stylesheet: file => calls.push(['unload', file.get_path()]),
        load_stylesheet: file => {
            calls.push(['load', file.get_path()]);
            return true;
        },
    };

    assert.deepEqual(reloadCustomStylesheets(theme), { ok: true, reloaded: 2, errors: [] });
    assert.deepEqual(calls, [
        ['unload', '/tmp/one.css'],
        ['load', '/tmp/one.css'],
        ['unload', '/tmp/two.css'],
        ['load', '/tmp/two.css'],
    ]);
});

test('reports stylesheet errors and continues with remaining files', () => {
    const files = [stylesheet('/tmp/broken.css'), stylesheet('/tmp/valid.css')];
    const theme = {
        get_custom_stylesheets: () => files,
        unload_stylesheet() {},
        load_stylesheet(file) {
            if (file.get_path() === '/tmp/broken.css')
                throw new Error('Invalid CSS');
            return true;
        },
    };

    assert.deepEqual(reloadCustomStylesheets(theme), {
        ok: false,
        reloaded: 1,
        errors: [{ file: '/tmp/broken.css', message: 'Invalid CSS' }],
    });
});

test('handles an empty custom stylesheet list', () => {
    const theme = {
        get_custom_stylesheets: () => [],
        unload_stylesheet() {},
        load_stylesheet() { return true; },
    };

    assert.deepEqual(reloadCustomStylesheets(theme), { ok: true, reloaded: 0, errors: [] });
});
