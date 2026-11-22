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
