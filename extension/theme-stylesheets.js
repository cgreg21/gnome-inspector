export function reloadCustomStylesheets(theme) {
    const files = theme.get_custom_stylesheets();
    const errors = [];
    let reloaded = 0;

    for (const file of files) {
        try {
            theme.unload_stylesheet(file);
            if (theme.load_stylesheet(file) === false)
                throw new Error('GNOME Shell rejected the stylesheet.');
            reloaded++;
        } catch (error) {
            errors.push({
                file: file.get_path?.() || file.get_uri?.() || 'Unknown stylesheet',
                message: error?.message || String(error),
            });
        }
    }

    return { ok: errors.length === 0, reloaded, errors };
}

export function customStylesheetPath(GLib) {
    return GLib.build_filenamev([
        GLib.get_user_config_dir(),
        'gnome-inspector',
        'custom.css',
    ]);
}

export function readCustomStylesheet(Gio, GLib) {
    const file = Gio.File.new_for_path(customStylesheetPath(GLib));
    if (!file.query_exists(null))
        return '';
    const [ok, contents] = file.load_contents(null);
    if (!ok)
        throw new Error('Could not read the custom stylesheet.');
    return new TextDecoder().decode(contents);
}

export function saveCustomStylesheet(theme, Gio, GLib, css) {
    if (typeof css !== 'string' || css.length > 65536)
        return { ok: false, error: 'CSS must be a string of at most 65536 characters.' };

    const path = customStylesheetPath(GLib);
    const directory = Gio.File.new_for_path(GLib.path_get_dirname(path));
    directory.make_directory_with_parents(null);
    const file = Gio.File.new_for_path(path);
    file.replace_contents(
        new TextEncoder().encode(css),
        null,
        false,
        Gio.FileCreateFlags.REPLACE_DESTINATION,
        null
    );

    const existing = theme.get_custom_stylesheets()
        .find(item => item.get_path?.() === path);
    if (existing)
        theme.unload_stylesheet(existing);

    if (theme.load_stylesheet(file) === false)
        return { ok: false, error: 'GNOME Shell rejected the custom stylesheet.', path };

    return { ok: true, path };
}
