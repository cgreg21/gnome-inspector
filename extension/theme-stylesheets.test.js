import assert from 'node:assert/strict';
import test from 'node:test';
import {
    customStylesheetPath,
    readCustomStylesheet,
    reloadCustomStylesheets,
    saveCustomStylesheet,
} from './theme-stylesheets.js';

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
        ['unload', '/tmp/one.css'], ['load', '/tmp/one.css'],
        ['unload', '/tmp/two.css'], ['load', '/tmp/two.css'],
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
        ok: false, reloaded: 1,
        errors: [{ file: '/tmp/broken.css', message: 'Invalid CSS' }],
    });
});

test('handles an empty custom stylesheet list', () => {
    const theme = { get_custom_stylesheets: () => [], unload_stylesheet() {}, load_stylesheet() { return true; } };
    assert.deepEqual(reloadCustomStylesheets(theme), { ok: true, reloaded: 0, errors: [] });
});

test('uses the user config directory for the stylesheet', () => {
    const GLib = {
        get_user_config_dir: () => '/home/test/.config',
        build_filenamev: parts => parts.join('/'),
    };
    assert.equal(customStylesheetPath(GLib), '/home/test/.config/gnome-inspector/custom.css');
});

test('rejects oversized CSS before writing', () => {
    let writes = 0;
    const Gio = { File: { new_for_path: () => ({ query_exists: () => true, replace_contents: () => writes++ }) } };
    const GLib = {
        get_user_config_dir: () => '/config',
        build_filenamev: parts => parts.join('/'),
        path_get_dirname: () => '/config/gnome-inspector',
    };
    assert.equal(saveCustomStylesheet({}, Gio, GLib, 'x'.repeat(65537)).ok, false);
    assert.equal(writes, 0);
});

test('reads an existing stylesheet', () => {
    const Gio = { File: { new_for_path: () => ({
        query_exists: () => true,
        load_contents: () => { throw new Error('Use GLib.file_get_contents'); },
    }) } };
    const GLib = {
        get_user_config_dir: () => '/config',
        build_filenamev: parts => parts.join('/'),
        file_get_contents: () => [true, new TextEncoder().encode('/* user css */')],
    };
    assert.equal(readCustomStylesheet(Gio, GLib), '/* user css */');
});

test('saves and applies a user stylesheet', () => {
    const writes = [];
    const path = '/config/gnome-inspector/custom.css';
    const Gio = {
        FileCreateFlags: {},
        File: { new_for_path: value => ({
            query_exists: () => value === path,
            make_directory_with_parents() {},
            get_path: () => value,
        }) },
    };
    const GLib = {
        get_user_config_dir: () => '/config',
        build_filenamev: parts => parts.join('/'),
        path_get_dirname: () => '/config/gnome-inspector',
        file_get_contents: () => [true, new TextEncoder().encode('old css')],
        file_set_contents: (file, css) => writes.push([file, css]),
    };
    const theme = {
        get_custom_stylesheets: () => [],
        unload_stylesheet() {},
        load_stylesheet: () => true,
    };
    assert.deepEqual(saveCustomStylesheet(theme, Gio, GLib, 'new css'), { ok: true, path });
    assert.deepEqual(writes, [[path, 'new css']]);
});

test('restores the previous stylesheet when GNOME rejects new CSS', () => {
    const writes = [];
    const path = '/config/gnome-inspector/custom.css';
    const Gio = {
        File: { new_for_path: value => ({
            query_exists: () => value === path,
            make_directory_with_parents() {},
            get_path: () => value,
        }) },
    };
    const GLib = {
        get_user_config_dir: () => '/config',
        build_filenamev: parts => parts.join('/'),
        path_get_dirname: () => '/config/gnome-inspector',
        file_get_contents: () => [true, new TextEncoder().encode('old css')],
        file_set_contents: (file, css) => writes.push([file, css]),
    };
    let attempts = 0;
    const theme = {
        get_custom_stylesheets: () => [],
        unload_stylesheet() {},
        load_stylesheet: () => ++attempts > 1,
    };
    const result = saveCustomStylesheet(theme, Gio, GLib, 'broken css');
    assert.equal(result.ok, false);
    assert.deepEqual(writes, [[path, 'broken css'], [path, 'old css']]);
    assert.equal(attempts, 2);
});
