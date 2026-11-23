use adw::prelude::*;
use glib::variant::ToVariant;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

const BUS_NAME: &str = "io.github.cgreg21.GnomeInspector.Shell";
const OBJECT_PATH: &str = "/io/github/cgreg21/GnomeInspector";
const INTERFACE: &str = "io.github.cgreg21.GnomeInspector";

fn shell_call(method: &str) -> Result<glib::Variant, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;

    connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            method,
            None,
            None,
            gio::DBusCallFlags::NONE,
            1_500,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())
}

fn string_reply(reply: &glib::Variant, error: &str) -> Result<String, String> {
    reply
        .get::<(String,)>()
        .map(|(value,)| value)
        .ok_or_else(|| error.to_string())
}

fn format_computed_style(json: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "Computed style is not an object.".to_string())?;
    Ok(object
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn filter_computed_style(report: &str, query: &str) -> String {
    if query.trim().is_empty() {
        return report.to_string();
    }
    let query = query.to_lowercase();
    report
        .lines()
        .filter(|line| line.to_lowercase().contains(&query))
        .collect::<Vec<_>>()
        .join("\n")
}

fn shell_version() -> Result<String, String> {
    string_reply(&shell_call("Ping")?, "Invalid Shell version reply.")
}

fn actor_tree() -> Result<String, String> {
    string_reply(&shell_call("GetActorTree")?, "Invalid actor tree reply.")
}

fn style_tree() -> Result<String, String> {
    string_reply(&shell_call("GetStyleTree")?, "Invalid style tree reply.")
}

fn actor_computed_style(path: &str) -> Result<String, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;
    let parameters = (path.to_string(),).to_variant();
    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "GetActorComputedStyle",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            1_500,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())?;
    string_reply(&reply, "Invalid computed CSS reply.")
}

fn set_actor_style_class(path: &str, style_class: &str) -> Result<String, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;
    let parameters = (path.to_string(), style_class.to_string()).to_variant();
    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "SetActorStyleClass",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            1_500,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())?;
    string_reply(&reply, "Invalid CSS class update reply.")
}

fn set_actor_property(path: &str, name: &str, value: &str) -> Result<serde_json::Value, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;
    let parameters = (path.to_string(), name.to_string(), value.to_string()).to_variant();
    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "SetActorProperty",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            1_500,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())?;
    let json = string_reply(&reply, "Invalid GObject property update reply.")?;
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

fn editable_actor_properties(node: &serde_json::Value) -> Vec<(String, String)> {
    node.get("properties")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|property| {
            property
                .get("writable")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
                && property
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|kind| {
                        matches!(
                            kind,
                            "gchararray"
                                | "gboolean"
                                | "gint"
                                | "guint"
                                | "gint64"
                                | "guint64"
                                | "gfloat"
                                | "gdouble"
                        )
                    })
        })
        .filter_map(|property| {
            let name = property.get("name")?.as_str()?.to_string();
            let value = property
                .get("value")
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| value.to_string())
                })
                .unwrap_or_default();
            Some((name, value))
        })
        .collect()
}

fn theme_info() -> Result<String, String> {
    string_reply(
        &shell_call("GetThemeInfo")?,
        "Invalid theme information reply.",
    )
}

fn custom_stylesheet() -> Result<String, String> {
    string_reply(
        &shell_call("GetCustomStylesheet")?,
        "Invalid custom stylesheet reply.",
    )
}

fn save_custom_stylesheet(css: &str) -> Result<serde_json::Value, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;
    let parameters = (css.to_string(),).to_variant();
    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "SaveCustomStylesheet",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            5_000,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())?;
    let json = string_reply(&reply, "Invalid custom stylesheet save reply.")?;
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

fn reload_custom_stylesheets() -> Result<serde_json::Value, String> {
    let reply = shell_call("ReloadCustomStylesheets")?;
    let json = string_reply(&reply, "Invalid stylesheet reload reply.")?;
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

fn set_actor_inline_style(path: &str, css: &str) -> Result<String, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;
    let parameters = (path.to_string(), css.to_string()).to_variant();
    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "SetActorInlineStyle",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            1_500,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())?;
    string_reply(&reply, "Invalid CSS update reply.")
}

fn diagnostics() -> Result<String, String> {
    string_reply(&shell_call("GetDiagnostics")?, "Invalid diagnostics reply.")
}

fn performance() -> Result<String, String> {
    string_reply(&shell_call("GetPerformance")?, "Invalid performance reply.")
}

fn actor_picker_call(method: &str) -> Result<String, String> {
    string_reply(&shell_call(method)?, "Invalid actor picker reply.")
}

fn start_actor_picker() -> Result<(), String> {
    let reply = actor_picker_call("StartActorPicker")?;
    let result: serde_json::Value =
        serde_json::from_str(&reply).map_err(|error| error.to_string())?;
    if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err("Impossible de démarrer la sélection visuelle.".to_string())
    }
}

fn actor_picker_result() -> Result<serde_json::Value, String> {
    let reply = actor_picker_call("GetActorPickerResult")?;
    serde_json::from_str(&reply).map_err(|error| error.to_string())
}

fn format_console_entry(source: &str, result: &str) -> String {
    format!("> {source}\n{result}\n\n")
}

fn remember_console_command(history: &mut Vec<String>, source: &str) {
    if history.last().is_some_and(|last| last == source) {
        return;
    }
    history.push(source.to_string());
    if history.len() > 50 {
        history.remove(0);
    }
}

fn evaluate_gjs(source: &str) -> Result<String, String> {
    let connection = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|error| error.to_string())?;
    let parameters = (source.to_string(),).to_variant();
    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "EvaluateGjs",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            10_000,
            None::<&gio::Cancellable>,
        )
        .map_err(|error| error.to_string())?;
    string_reply(&reply, "Invalid GJS console reply.")
}

fn page_heading(title: &str, description: &str) -> gtk::Box {
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 6);
    heading.set_margin_top(24);
    heading.set_margin_bottom(12);
    heading.set_margin_start(24);
    heading.set_margin_end(24);

    let title = gtk::Label::new(Some(title));
    title.add_css_class("title-1");
    title.set_halign(gtk::Align::Start);

    let description = gtk::Label::new(Some(description));
    description.add_css_class("dim-label");
    description.set_halign(gtk::Align::Start);
    description.set_wrap(true);

    heading.append(&title);
    heading.append(&description);
    heading
}

fn extension_is_discovered(output: &str, uuid: &str) -> bool {
    output.lines().any(|line| line.trim() == uuid)
}

fn install_extension() -> Result<(), String> {
    let uuid = "gnome-inspector@cgreg21";
    let data_dir = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".local/share"))
        })
        .ok_or_else(|| "Impossible de déterminer le dossier de données utilisateur.".to_string())?;
    let extension_dir = data_dir.join("gnome-shell/extensions").join(uuid);

    std::fs::create_dir_all(&extension_dir).map_err(|error| error.to_string())?;
    for (name, contents) in [
        ("metadata.json", include_str!("../extension/metadata.json")),
        ("extension.js", include_str!("../extension/extension.js")),
        ("actor-tree.js", include_str!("../extension/actor-tree.js")),
        (
            "property-editor.js",
            include_str!("../extension/property-editor.js"),
        ),
        (
            "actor-picker.js",
            include_str!("../extension/actor-picker.js"),
        ),
        ("style-tree.js", include_str!("../extension/style-tree.js")),
        (
            "theme-stylesheets.js",
            include_str!("../extension/theme-stylesheets.js"),
        ),
    ] {
        std::fs::write(extension_dir.join(name), contents).map_err(|error| error.to_string())?;
    }

    let listed = Command::new("gnome-extensions")
        .arg("list")
        .output()
        .map_err(|error| format!("Impossible de vérifier les extensions GNOME : {error}"))?;
    if !listed.status.success() {
        return Err(format!(
            "Impossible de vérifier les extensions GNOME : {}",
            String::from_utf8_lossy(&listed.stderr).trim()
        ));
    }
    if !extension_is_discovered(&String::from_utf8_lossy(&listed.stdout), uuid) {
        return Err(format!(
            "Extension copiée dans {} mais GNOME Shell ne l'a pas encore découverte. Déconnectez-vous puis reconnectez-vous, puis relancez GNOME Inspector.",
            extension_dir.display()
        ));
    }

    let output = Command::new("gnome-extensions")
        .arg("enable")
        .arg(uuid)
        .output()
        .map_err(|error| format!("Impossible de lancer gnome-extensions : {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Activation refusée : {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

fn refresh_connection(status: &gtk::Label, setup: &gtk::Box) {
    match shell_version() {
        Ok(version) => {
            status.set_text(&format!("Connecté à GNOME Shell {version}"));
            status.remove_css_class("error");
            status.add_css_class("success");
            setup.set_visible(false);
        }
        Err(connection_error) => {
            status.set_text("Installation et activation automatiques de l’extension…");
            status.remove_css_class("error");
            status.remove_css_class("success");
            match install_extension() {
                Ok(()) => {
                    status.set_text("Extension installée. Attente de la connexion à GNOME Shell…");
                    setup.set_visible(false);
                    let status_retry = status.clone();
                    let setup_retry = setup.clone();
                    let mut attempts = 5;
                    glib::timeout_add_seconds_local(2, move || match shell_version() {
                        Ok(version) => {
                            status_retry.set_text(&format!("Connecté à GNOME Shell {version}"));
                            status_retry.remove_css_class("error");
                            status_retry.add_css_class("success");
                            setup_retry.set_visible(false);
                            glib::ControlFlow::Break
                        }
                        Err(error) => {
                            attempts -= 1;
                            if attempts == 0 {
                                status_retry.set_text(&format!(
                                    "Extension installée, mais connexion impossible — {error}"
                                ));
                                status_retry.add_css_class("error");
                                setup_retry.set_visible(true);
                                glib::ControlFlow::Break
                            } else {
                                glib::ControlFlow::Continue
                            }
                        }
                    });
                }
                Err(error) => {
                    status.set_text(&format!(
                        "Installation automatique impossible — {error} (connexion : {connection_error})"
                    ));
                    status.add_css_class("error");
                    setup.set_visible(true);
                }
            }
        }
    }
}

fn flatten_actors(
    node: &serde_json::Value,
    depth: usize,
    rows: &mut Vec<(serde_json::Value, usize)>,
) {
    rows.push((node.clone(), depth));
    if let Some(children) = node.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            flatten_actors(child, depth + 1, rows);
        }
    }
}

fn actor_matches(node: &serde_json::Value, query: &str) -> bool {
    let matches_identity = ["name", "type"]
        .iter()
        .filter_map(|key| node.get(*key).and_then(serde_json::Value::as_str))
        .any(|value| value.to_lowercase().contains(query));
    let matches_properties = node
        .get("properties")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|properties| {
            properties.iter().any(|property| {
                ["name", "value"]
                    .iter()
                    .filter_map(|key| property.get(*key))
                    .any(|value| {
                        let value = value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string());
                        value.to_lowercase().contains(query)
                    })
            })
        });

    matches_identity
        || matches_properties
        || node
            .get("children")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|children| children.iter().any(|child| actor_matches(child, query)))
}

fn actor_style_label(node: &serde_json::Value, depth: usize) -> String {
    let name = node
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Acteur");
    let classes = node
        .get("styleClass")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let path = actor_path(node).unwrap_or("");
    format!(
        "{}{}  ·  {}  ·  {}",
        "  ".repeat(depth),
        name,
        classes,
        path
    )
}

fn actor_path(node: &serde_json::Value) -> Option<&str> {
    node.get("path").and_then(serde_json::Value::as_str)
}

fn actor_index_by_path(nodes: &[serde_json::Value], path: &str) -> Option<usize> {
    nodes.iter().position(|node| actor_path(node) == Some(path))
}

fn actor_selection_index(
    nodes: &[serde_json::Value],
    previous_path: Option<&str>,
) -> Option<usize> {
    previous_path
        .and_then(|path| actor_index_by_path(nodes, path))
        .or_else(|| (!nodes.is_empty()).then_some(0))
}

fn actor_detail_summary(node: &serde_json::Value) -> serde_json::Value {
    let mut details = serde_json::Map::new();
    for key in [
        "path",
        "name",
        "type",
        "visible",
        "reactive",
        "x",
        "y",
        "width",
        "height",
        "properties",
        "propertyCount",
        "propertiesTruncated",
        "propertyErrors",
        "propertiesError",
        "childrenError",
        "actorError",
    ] {
        if let Some(value) = node.get(key) {
            details.insert(key.to_string(), value.clone());
        }
    }
    serde_json::Value::Object(details)
}

#[cfg(test)]
fn format_actor_details(node: &serde_json::Value) -> String {
    format_actor_details_filtered(node, "")
}

fn format_actor_details_filtered(node: &serde_json::Value, query: &str) -> String {
    let query = query.trim().to_lowercase();
    let summary = actor_detail_summary(node);
    let mut lines = Vec::new();

    for key in [
        "name", "type", "path", "visible", "reactive", "x", "y", "width", "height",
    ] {
        if let Some(value) = summary.get(key) {
            lines.push(format!("{key}: {value}"));
        }
    }

    if let Some(properties) = summary
        .get("properties")
        .and_then(serde_json::Value::as_array)
    {
        lines.push(String::new());
        let matching = properties
            .iter()
            .filter(|property| {
                if query.is_empty() {
                    return true;
                }
                let searchable = [
                    property
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                    property
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                    property
                        .get("value")
                        .map(serde_json::Value::to_string)
                        .as_deref()
                        .unwrap_or(""),
                ]
                .join(" ")
                .to_lowercase();
                searchable.contains(&query)
            })
            .collect::<Vec<_>>();
        lines.push(if query.is_empty() {
            format!("Propriétés ({})", properties.len())
        } else {
            format!("Propriétés ({}/{})", matching.len(), properties.len())
        });
        for property in matching {
            let name = property
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?");
            let kind = property
                .get("type")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("?");
            let access = [
                (
                    "R",
                    property
                        .get("readable")
                        .and_then(serde_json::Value::as_bool),
                ),
                (
                    "W",
                    property
                        .get("writable")
                        .and_then(serde_json::Value::as_bool),
                ),
            ]
            .iter()
            .filter_map(|(flag, enabled)| (*enabled == Some(true)).then_some(*flag))
            .collect::<Vec<_>>()
            .join(",");

            let value = property.get("value").unwrap_or(&serde_json::Value::Null);
            let readable = property
                .get("error")
                .and_then(serde_json::Value::as_str)
                .map(|error| format!("<erreur: {error}>"))
                .unwrap_or_else(|| {
                    if property
                        .get("readable")
                        .and_then(serde_json::Value::as_bool)
                        == Some(false)
                    {
                        "<non lisible>".to_string()
                    } else {
                        value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string())
                    }
                });
            let readable = readable.replace('\n', "\\n");
            lines.push(format!("{name} ({kind}; {access}) = {readable}"));
        }
    }

    for key in [
        "propertyCount",
        "propertiesTruncated",
        "propertyErrors",
        "propertiesError",
        "childrenError",
        "actorError",
    ] {
        if let Some(value) = summary.get(key) {
            lines.push(format!("{key}: {value}"));
        }
    }

    lines.join("\n")
}

fn show_actor_details(view: &gtk::TextView, node: &serde_json::Value) {
    show_actor_details_filtered(view, node, "");
}

fn show_actor_details_filtered(view: &gtk::TextView, node: &serde_json::Value, query: &str) {
    view.buffer()
        .set_text(&format_actor_details_filtered(node, query));
}

fn refresh_actor_tree(
    list: &gtk::ListBox,
    details: &gtk::TextView,
    selected_nodes: &Rc<RefCell<Vec<serde_json::Value>>>,
    status: &gtk::Label,
    expander_widgets: &Rc<RefCell<Vec<gtk::ToggleButton>>>,
) {
    match actor_tree() {
        Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(snapshot) => {
                let previous_path = list
                    .selected_row()
                    .and_then(|row| row.widget_name().parse::<usize>().ok())
                    .and_then(|index| {
                        selected_nodes
                            .borrow()
                            .get(index)
                            .and_then(actor_path)
                            .map(str::to_owned)
                    });
                let mut rows = Vec::new();
                if let Some(root) = snapshot.get("root") {
                    flatten_actors(root, 0, &mut rows);
                }

                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }

                let nodes = rows
                    .iter()
                    .map(|(node, _)| node.clone())
                    .collect::<Vec<_>>();
                *selected_nodes.borrow_mut() = nodes;

                let depths = rows.iter().map(|(_, depth)| *depth).collect::<Vec<_>>();
                let mut parents = vec![None; rows.len()];
                let mut ancestors = Vec::<usize>::new();
                for (index, depth) in depths.iter().enumerate() {
                    while ancestors.len() > *depth {
                        ancestors.pop();
                    }
                    if *depth > 0 {
                        parents[index] = ancestors.last().copied();
                    }
                    ancestors.push(index);
                }

                let expanded = Rc::new(RefCell::new(
                    depths.iter().map(|depth| *depth == 0).collect::<Vec<_>>(),
                ));
                expander_widgets.borrow_mut().clear();
                let row_widgets = Rc::new(RefCell::new(Vec::<gtk::ListBoxRow>::new()));

                for (index, (node, depth)) in rows.into_iter().enumerate() {
                    let name = node
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Acteur");
                    let kind = node
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Unknown");
                    let has_children = node
                        .get("children")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|children| !children.is_empty());

                    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    content.set_margin_top(3);
                    content.set_margin_bottom(3);
                    content.set_margin_start(6 + depth as i32 * 14);
                    content.set_margin_end(8);

                    if has_children {
                        let expander = gtk::ToggleButton::with_label("▸");
                        expander.set_active(depth == 0);
                        expander_widgets.borrow_mut().push(expander.clone());
                        expander.set_has_frame(false);
                        expander.set_size_request(24, 24);
                        expander.set_tooltip_text(Some("Déplier ou replier les enfants"));
                        let expanded_for_toggle = expanded.clone();
                        let parents_for_toggle = parents.clone();
                        let rows_for_toggle = row_widgets.clone();
                        expander.connect_toggled(move |button| {
                            let mut state = expanded_for_toggle.borrow_mut();
                            state[index] = button.is_active();
                            button.set_label(if button.is_active() { "▾" } else { "▸" });
                            let widgets = rows_for_toggle.borrow();
                            for (row_index, row) in widgets.iter().enumerate() {
                                let mut parent = parents_for_toggle[row_index];
                                let mut visible = true;
                                while let Some(parent_index) = parent {
                                    if !state[parent_index] {
                                        visible = false;
                                        break;
                                    }
                                    parent = parents_for_toggle[parent_index];
                                }
                                row.set_visible(visible);
                            }
                        });
                        content.append(&expander);
                    } else {
                        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                        spacer.set_size_request(24, -1);
                        content.append(&spacer);
                    }

                    let label = gtk::Label::new(Some(&format!("{name}  ·  {kind}")));
                    label.set_halign(gtk::Align::Start);
                    label.set_hexpand(true);
                    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    content.append(&label);

                    let row = gtk::ListBoxRow::new();
                    row.set_widget_name(&index.to_string());
                    row.set_child(Some(&content));
                    let mut parent = parents[index];
                    let mut visible = true;
                    while let Some(parent_index) = parent {
                        if !expanded.borrow()[parent_index] {
                            visible = false;
                            break;
                        }
                        parent = parents[parent_index];
                    }
                    row.set_visible(visible);
                    list.append(&row);
                    row_widgets.borrow_mut().push(row);
                }

                let actor_count = snapshot
                    .get("actorCount")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(rows_len(selected_nodes));
                let incomplete = snapshot
                    .get("incomplete")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let truncated = snapshot
                    .get("truncated")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                status.set_text(&format!(
                    "{actor_count} acteurs · {}{}",
                    if incomplete {
                        "capture incomplète"
                    } else {
                        "capture complète"
                    },
                    if truncated {
                        " · limite de parcours atteinte"
                    } else {
                        ""
                    }
                ));

                let selected_index =
                    actor_selection_index(&selected_nodes.borrow(), previous_path.as_deref());
                if let Some(index) = selected_index
                    && let Some(row) = list.row_at_index(index as i32)
                {
                    list.select_row(Some(&row));
                } else {
                    list.unselect_all();
                    details.buffer().set_text("Aucun acteur dans la capture.");
                }
                status.remove_css_class("error");
            }
            Err(error) => {
                status.set_text(&format!("Réponse JSON invalide : {error}"));
                status.add_css_class("error");
            }
        },
        Err(error) => {
            status.set_text(&format!("Échec de l’inspection : {error}"));
            status.add_css_class("error");
        }
    }
}

fn rows_len(nodes: &Rc<RefCell<Vec<serde_json::Value>>>) -> u64 {
    nodes.borrow().len() as u64
}

fn actors_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(
        "Acteurs",
        "Parcourez la hiérarchie Clutter/St et sélectionnez un acteur pour examiner ses propriétés.",
    ));

    let status = gtk::Label::new(Some("Aucune capture effectuée."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(24);
    status.add_css_class("dim-label");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("navigation-sidebar");
    let list_scroll = gtk::ScrolledWindow::builder()
        .min_content_width(300)
        .max_content_width(420)
        .vexpand(true)
        .child(&list)
        .build();

    let details = gtk::TextView::new();
    details.set_editable(false);
    details.set_cursor_visible(false);
    details.set_monospace(true);
    details.set_wrap_mode(gtk::WrapMode::WordChar);
    details.set_left_margin(12);
    details.set_top_margin(12);
    details
        .buffer()
        .set_text("Sélectionnez un acteur pour afficher ses propriétés.");
    let details_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&details)
        .build();
    let property_search = gtk::SearchEntry::new();
    property_search.set_placeholder_text(Some("Filtrer les propriétés par nom, type ou valeur…"));
    property_search.set_margin_start(12);
    property_search.set_margin_end(12);
    let property_editor = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    property_editor.set_margin_start(12);
    property_editor.set_margin_end(12);
    let property_name = gtk::ComboBoxText::new();
    property_name.set_hexpand(true);
    property_name.set_sensitive(false);
    let property_value = gtk::Entry::new();
    property_value.set_hexpand(true);
    property_value.set_placeholder_text(Some("Nouvelle valeur"));
    property_value.set_sensitive(false);
    let apply_property = gtk::Button::with_label("Appliquer");
    apply_property.set_sensitive(false);
    property_editor.append(&property_name);
    property_editor.append(&property_value);
    property_editor.append(&apply_property);

    let details_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    details_panel.append(&property_search);
    details_panel.append(&property_editor);
    details_panel.append(&details_scroll);

    let selected_actor = Rc::new(RefCell::new(None::<serde_json::Value>));
    let selected_nodes = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
    let query = Rc::new(RefCell::new(String::new()));
    let expander_widgets = Rc::new(RefCell::new(Vec::<gtk::ToggleButton>::new()));
    let query_for_filter = query.clone();
    let nodes_for_filter = selected_nodes.clone();
    list.set_filter_func(move |row| {
        let query = query_for_filter.borrow();
        if query.is_empty() {
            return true;
        }
        row.widget_name()
            .parse::<usize>()
            .ok()
            .and_then(|index| nodes_for_filter.borrow().get(index).cloned())
            .is_some_and(|node| actor_matches(&node, query.as_str()))
    });

    let nodes_for_selection = selected_nodes.clone();
    let actor_for_selection = selected_actor.clone();
    let details_for_selection = details.clone();
    let search_for_selection = property_search.clone();
    let property_name_for_selection = property_name.clone();
    let property_value_for_selection = property_value.clone();
    let apply_for_selection = apply_property.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row
            && let Ok(index) = row.widget_name().parse::<usize>()
            && let Some(node) = nodes_for_selection.borrow().get(index)
        {
            *actor_for_selection.borrow_mut() = Some(node.clone());
            show_actor_details_filtered(
                &details_for_selection,
                node,
                search_for_selection.text().as_str(),
            );
            property_name_for_selection.remove_all();
            let editable = editable_actor_properties(node);
            for (name, _) in &editable {
                property_name_for_selection.append(Some(name), name);
            }
            property_name_for_selection.set_sensitive(!editable.is_empty());
            property_value_for_selection.set_sensitive(!editable.is_empty());
            apply_for_selection.set_sensitive(!editable.is_empty());
            if let Some((_, value)) = editable.first() {
                property_name_for_selection.set_active(Some(0));
                property_value_for_selection.set_text(value);
            } else {
                property_value_for_selection.set_text("");
            }
        }
    });

    let actor_for_property_search = selected_actor.clone();
    let details_for_property_search = details.clone();
    property_search.connect_search_changed(move |entry| {
        if let Some(node) = actor_for_property_search.borrow().as_ref() {
            show_actor_details_filtered(&details_for_property_search, node, entry.text().as_str());
        }
    });

    let actor_for_property_choice = selected_actor.clone();
    let value_for_property_choice = property_value.clone();
    let name_for_property_choice = property_name.clone();
    property_name.connect_changed(move |_| {
        let Some(node) = actor_for_property_choice.borrow().as_ref().cloned() else {
            return;
        };
        let Some(name) = name_for_property_choice.active_id() else {
            return;
        };
        if let Some((_, value)) = editable_actor_properties(&node)
            .into_iter()
            .find(|(candidate, _)| candidate == name.as_str())
        {
            value_for_property_choice.set_text(&value);
        }
    });

    let actor_for_apply = selected_actor.clone();
    let name_for_apply = property_name.clone();
    let value_for_apply = property_value.clone();
    let status_for_apply = status.clone();
    let details_for_apply = details.clone();
    let search_for_apply = property_search.clone();
    apply_property.connect_clicked(move |_| {
        let Some(actor) = actor_for_apply.borrow().as_ref().cloned() else {
            status_for_apply.set_text("Sélectionnez un acteur.");
            return;
        };
        let Some(path) = actor_path(&actor) else {
            status_for_apply.set_text("Chemin d’acteur indisponible.");
            return;
        };
        let Some(name) = name_for_apply.active_id() else {
            status_for_apply.set_text("Aucune propriété modifiable sélectionnée.");
            return;
        };
        match set_actor_property(path, name.as_str(), value_for_apply.text().as_str()) {
            Ok(result) if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) => {
                status_for_apply.set_text(&format!("Propriété « {name} » mise à jour."));
                status_for_apply.remove_css_class("error");
                let mut updated = actor;
                if let Some(properties) = updated
                    .get_mut("properties")
                    .and_then(serde_json::Value::as_array_mut)
                    && let Some(property) = properties.iter_mut().find(|property| {
                        property.get("name").and_then(serde_json::Value::as_str)
                            == Some(name.as_str())
                    })
                    && let Some(value) = result.get("value")
                {
                    property["value"] = value.clone();
                }
                *actor_for_apply.borrow_mut() = Some(updated.clone());
                show_actor_details_filtered(
                    &details_for_apply,
                    &updated,
                    search_for_apply.text().as_str(),
                );
            }
            Ok(result) => {
                let error = result
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Modification refusée.");
                status_for_apply.set_text(error);
                status_for_apply.add_css_class("error");
            }
            Err(error) => {
                status_for_apply.set_text(&format!("Échec de la modification : {error}"));
                status_for_apply.add_css_class("error");
            }
        }
    });

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Rechercher un acteur ou un type…"));
    search.set_hexpand(true);
    let query_for_search = query.clone();
    let list_for_search = list.clone();
    let expanders_for_search = expander_widgets.clone();
    search.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        *query_for_search.borrow_mut() = query.clone();
        for (index, expander) in expanders_for_search.borrow().iter().enumerate() {
            expander.set_active(!query.is_empty() || index == 0);
        }
        list_for_search.invalidate_filter();
    });

    let copy_path = gtk::Button::with_label("Copier le chemin");
    copy_path.set_sensitive(false);
    let list_for_copy = list.clone();
    let nodes_for_copy = selected_nodes.clone();
    let status_for_copy = status.clone();
    let copy_path_state = copy_path.clone();
    list.connect_row_selected(move |_, row| copy_path_state.set_sensitive(row.is_some()));
    copy_path.connect_clicked(move |_| {
        let path = list_for_copy
            .selected_row()
            .and_then(|row| row.widget_name().parse::<usize>().ok())
            .and_then(|index| {
                nodes_for_copy
                    .borrow()
                    .get(index)
                    .and_then(actor_path)
                    .map(str::to_owned)
            });

        if let Some(path) = path {
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&path);
                status_for_copy.set_text(&format!("Chemin copié · {path}"));
                status_for_copy.remove_css_class("error");
            } else {
                status_for_copy.set_text("Presse-papiers indisponible.");
                status_for_copy.add_css_class("error");
            }
        } else {
            status_for_copy.set_text("Sélectionnez d’abord un acteur.");
        }
    });

    let pick_actor = gtk::Button::with_label("Sélectionner dans Shell");
    let list_for_picker = list.clone();
    let nodes_for_picker = selected_nodes.clone();
    let status_for_picker = status.clone();
    pick_actor.connect_clicked(move |_| {
        match start_actor_picker() {
            Ok(()) => {
                status_for_picker.set_text("Cliquez sur un acteur dans GNOME Shell · Échap pour annuler.");
                let list = list_for_picker.clone();
                let nodes = nodes_for_picker.clone();
                let status = status_for_picker.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
                    match actor_picker_result() {
                        Ok(result) => match result.get("status").and_then(serde_json::Value::as_str) {
                            Some("pending") => glib::ControlFlow::Continue,
                            Some("selected") => {
                                let path = result.get("path").and_then(serde_json::Value::as_str).unwrap_or("");
                                let index = nodes.borrow().iter().position(|node| {
                                    node.get("path").and_then(serde_json::Value::as_str) == Some(path)
                                });
                                if let Some(index) = index
                                    && let Some(row) = list.row_at_index(index as i32)
                                {
                                    list.select_row(Some(&row));
                                    status.set_text(&format!("Acteur sélectionné · chemin {path}"));
                                } else {
                                    status.set_text("Acteur sélectionné dans Shell ; actualisez l’arbre pour synchroniser la ligne.");
                                }
                                glib::ControlFlow::Break
                            }
                            Some("cancelled") => {
                                status.set_text("Sélection visuelle annulée.");
                                glib::ControlFlow::Break
                            }
                            Some("timeout") => {
                                status.set_text("Délai de sélection visuelle dépassé.");
                                glib::ControlFlow::Break
                            }
                            _ => {
                                status.set_text("Sélection visuelle inactive.");
                                glib::ControlFlow::Break
                            }
                        },
                        Err(error) => {
                            status.set_text(&format!("Erreur de sélection visuelle : {error}"));
                            status.add_css_class("error");
                            glib::ControlFlow::Break
                        }
                    }
                });
            }
            Err(error) => {
                status_for_picker.set_text(&format!("Impossible de démarrer la sélection : {error}"));
                status_for_picker.add_css_class("error");
            }
        }
    });

    let refresh = gtk::Button::with_label("Actualiser");
    refresh.add_css_class("suggested-action");
    refresh.set_halign(gtk::Align::Start);
    let list_for_refresh = list.clone();
    let details_for_refresh = details.clone();
    let nodes_for_refresh = selected_nodes.clone();
    let status_for_refresh = status.clone();
    let expanders_for_refresh = expander_widgets.clone();
    refresh.connect_clicked(move |_| {
        refresh_actor_tree(
            &list_for_refresh,
            &details_for_refresh,
            &nodes_for_refresh,
            &status_for_refresh,
            &expanders_for_refresh,
        );
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_start(24);
    actions.set_margin_end(24);
    actions.append(&search);
    actions.append(&copy_path);
    actions.append(&pick_actor);
    actions.append(&refresh);

    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_start_child(Some(&list_scroll));
    split.set_end_child(Some(&details_panel));
    split.set_resize_start_child(false);
    split.set_shrink_start_child(false);
    split.set_position(340);

    page.append(&actions);
    page.append(&status);
    page.append(&split);
    page
}

fn styles_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(
        "CSS et thèmes",
        "Inspectez les acteurs, les classes CSS et les feuilles de style actives.",
    ));

    let status = gtk::Label::new(Some("Aucune capture CSS effectuée."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(24);
    status.add_css_class("dim-label");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("navigation-sidebar");
    let list_scroll = gtk::ScrolledWindow::builder()
        .min_content_width(300)
        .max_content_width(420)
        .vexpand(true)
        .child(&list)
        .build();

    let details = gtk::TextView::new();
    details.set_editable(false);
    details.set_cursor_visible(false);
    details.set_monospace(true);
    details.set_wrap_mode(gtk::WrapMode::WordChar);
    details.set_left_margin(12);
    details.set_top_margin(12);
    details
        .buffer()
        .set_text("Sélectionnez un acteur pour inspecter ses styles.");
    let details_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&details)
        .build();

    let selected_nodes = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
    let query = Rc::new(RefCell::new(String::new()));
    let query_for_filter = query.clone();
    list.set_filter_func(move |row| {
        let query = query_for_filter.borrow();
        if query.is_empty() {
            return true;
        }
        row.child()
            .and_then(|child| child.downcast::<gtk::Label>().ok())
            .is_some_and(|label| label.text().to_lowercase().contains(query.as_str()))
    });

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Rechercher un acteur ou une classe CSS…"));
    search.set_hexpand(true);
    let query_for_search = query.clone();
    let list_for_search = list.clone();
    search.connect_search_changed(move |entry| {
        *query_for_search.borrow_mut() = entry.text().to_lowercase();
        list_for_search.invalidate_filter();
    });

    let selected_path = Rc::new(RefCell::new(None::<String>));
    let class_input = gtk::Entry::new();
    class_input.set_placeholder_text(Some("Classes CSS : button primary"));
    class_input.set_hexpand(true);
    class_input.set_sensitive(false);
    let style_input = gtk::Entry::new();
    style_input.set_placeholder_text(Some("Style inline : color: white;"));
    style_input.set_hexpand(true);
    style_input.set_sensitive(false);

    let nodes_for_selection = selected_nodes.clone();
    let path_for_selection = selected_path.clone();
    let details_for_selection = details.clone();
    let class_for_selection = class_input.clone();
    let style_for_selection = style_input.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row
            && let Ok(index) = row.widget_name().parse::<usize>()
            && let Some(node) = nodes_for_selection.borrow().get(index)
        {
            show_actor_details(&details_for_selection, node);
            *path_for_selection.borrow_mut() = node
                .get("path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);
            class_for_selection.set_text(
                node.get("styleClass")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            );
            style_for_selection.set_text(
                node.get("inlineStyle")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            );
        }
    });

    let list_for_refresh = list.clone();
    let details_for_refresh = details.clone();
    let nodes_for_refresh = selected_nodes.clone();
    let path_for_refresh = selected_path.clone();
    let class_for_refresh = class_input.clone();
    let style_for_refresh = style_input.clone();
    let status_for_refresh = status.clone();
    let refresh = gtk::Button::with_label("Actualiser les styles");
    refresh.add_css_class("suggested-action");
    refresh.set_halign(gtk::Align::Start);
    refresh.connect_clicked(move |_| {
        let previous_path = path_for_refresh.borrow().clone();
        match style_tree().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(&json).map_err(|error| error.to_string())
        }) {
            Ok(snapshot) => {
                let mut rows = Vec::new();
                if let Some(root) = snapshot.get("root") {
                    flatten_actors(root, 0, &mut rows);
                }
                while let Some(child) = list_for_refresh.first_child() {
                    list_for_refresh.remove(&child);
                }

                let nodes = rows
                    .iter()
                    .map(|(node, _)| node.clone())
                    .collect::<Vec<_>>();
                *nodes_for_refresh.borrow_mut() = nodes;
                for (index, (node, depth)) in rows.into_iter().enumerate() {
                    let label = gtk::Label::new(Some(&actor_style_label(&node, depth)));
                    label.set_halign(gtk::Align::Start);
                    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    label.set_margin_top(5);
                    label.set_margin_bottom(5);
                    label.set_margin_start(8);
                    label.set_margin_end(8);
                    let row = gtk::ListBoxRow::new();
                    row.set_widget_name(&index.to_string());
                    row.set_child(Some(&label));
                    list_for_refresh.append(&row);
                }

                let count = snapshot
                    .get("actorCount")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let truncated = snapshot
                    .get("truncated")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                status_for_refresh.set_text(&format!(
                    "{count} acteurs inspectés{}",
                    if truncated {
                        " · capture tronquée"
                    } else {
                        ""
                    }
                ));
                let selected_index = previous_path
                    .as_deref()
                    .and_then(|path| actor_index_by_path(&nodes_for_refresh.borrow(), path))
                    .or_else(|| (!nodes_for_refresh.borrow().is_empty()).then_some(0));
                if let Some(index) = selected_index {
                    if let Some(row) = list_for_refresh.row_at_index(index as i32) {
                        list_for_refresh.select_row(Some(&row));
                    }
                } else {
                    *path_for_refresh.borrow_mut() = None;
                    class_for_refresh.set_text("");
                    style_for_refresh.set_text("");
                    details_for_refresh.buffer().set_text("Aucun style trouvé.");
                }
                status_for_refresh.remove_css_class("error");
            }
            Err(error) => {
                status_for_refresh.set_text(&format!("Échec de l’inspection CSS : {error}"));
                status_for_refresh.add_css_class("error");
            }
        }
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_start(24);
    actions.set_margin_end(24);
    actions.append(&search);
    actions.append(&refresh);

    let class_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    class_row.set_margin_start(24);
    class_row.set_margin_end(24);
    let apply_class = gtk::Button::with_label("Appliquer les classes");
    apply_class.set_sensitive(false);
    let class_for_apply = class_input.clone();
    let path_for_class = selected_path.clone();
    let status_for_class = status.clone();
    apply_class.connect_clicked(move |_| {
        let Some(path) = path_for_class.borrow().clone() else {
            status_for_class.set_text("Sélectionnez d’abord un acteur.");
            status_for_class.add_css_class("error");
            return;
        };
        match set_actor_style_class(&path, class_for_apply.text().as_str()) {
            Ok(reply) => match serde_json::from_str::<serde_json::Value>(&reply) {
                Ok(result)
                    if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) =>
                {
                    status_for_class.set_text("Classes CSS appliquées.");
                    status_for_class.remove_css_class("error");
                }
                Ok(result) => {
                    let error = result
                        .get("error")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Erreur inconnue");
                    status_for_class.set_text(&format!("Modification refusée : {error}"));
                    status_for_class.add_css_class("error");
                }
                Err(error) => {
                    status_for_class.set_text(&format!("Réponse invalide : {error}"));
                    status_for_class.add_css_class("error");
                }
            },
            Err(error) => {
                status_for_class.set_text(&format!("Échec de la modification CSS : {error}"));
                status_for_class.add_css_class("error");
            }
        }
    });
    class_row.append(&class_input);
    class_row.append(&apply_class);

    let style_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    style_row.set_margin_start(24);
    style_row.set_margin_end(24);
    let apply_style = gtk::Button::with_label("Appliquer le style inline");
    apply_style.set_sensitive(false);
    let style_for_apply = style_input.clone();
    let path_for_style = selected_path.clone();
    let status_for_style = status.clone();
    apply_style.connect_clicked(move |_| {
        let Some(path) = path_for_style.borrow().clone() else {
            status_for_style.set_text("Sélectionnez d’abord un acteur.");
            status_for_style.add_css_class("error");
            return;
        };
        match set_actor_inline_style(&path, style_for_apply.text().as_str()) {
            Ok(reply) => match serde_json::from_str::<serde_json::Value>(&reply) {
                Ok(result)
                    if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) =>
                {
                    status_for_style.set_text("Style inline appliqué.");
                    status_for_style.remove_css_class("error");
                }
                Ok(result) => {
                    let error = result
                        .get("error")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Erreur inconnue");
                    status_for_style.set_text(&format!("Modification refusée : {error}"));
                    status_for_style.add_css_class("error");
                }
                Err(error) => {
                    status_for_style.set_text(&format!("Réponse invalide : {error}"));
                    status_for_style.add_css_class("error");
                }
            },
            Err(error) => {
                status_for_style.set_text(&format!("Échec de la modification CSS : {error}"));
                status_for_style.add_css_class("error");
            }
        }
    });
    style_row.append(&style_input);
    style_row.append(&apply_style);

    let list_for_edit_state = list.clone();
    let class_for_edit_state = class_input.clone();
    let style_for_edit_state = style_input.clone();
    let apply_class_for_edit_state = apply_class.clone();
    let apply_style_for_edit_state = apply_style.clone();
    list_for_edit_state.connect_row_selected(move |_, row| {
        let enabled = row.is_some();
        class_for_edit_state.set_sensitive(enabled);
        style_for_edit_state.set_sensitive(enabled);
        apply_class_for_edit_state.set_sensitive(enabled);
        apply_style_for_edit_state.set_sensitive(enabled);
    });

    let computed_details = gtk::TextView::new();
    computed_details.set_editable(false);
    computed_details.set_cursor_visible(false);
    computed_details.set_monospace(true);
    computed_details.set_wrap_mode(gtk::WrapMode::WordChar);
    computed_details.set_left_margin(12);
    computed_details.set_top_margin(12);
    let computed_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(100)
        .max_content_height(160)
        .child(&computed_details)
        .build();
    let computed_report = Rc::new(RefCell::new(String::new()));
    let computed_search = gtk::SearchEntry::new();
    computed_search.set_placeholder_text(Some("Filtrer les propriétés calculées…"));
    let report_for_search = computed_report.clone();
    let view_for_search = computed_details.clone();
    computed_search.connect_search_changed(move |entry| {
        let report = report_for_search.borrow();
        let filtered = filter_computed_style(&report, entry.text().as_str());
        view_for_search.buffer().set_text(&filtered);
    });
    let computed_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    computed_actions.set_margin_start(24);
    let computed_refresh = gtk::Button::with_label("Lire le style calculé de l’acteur");
    let computed_view = computed_details.clone();
    let computed_path = selected_path.clone();
    let computed_status = status.clone();
    let report_for_refresh = computed_report.clone();
    let search_for_refresh = computed_search.clone();
    computed_refresh.connect_clicked(move |_| {
        let Some(path) = computed_path.borrow().clone() else {
            computed_status.set_text("Sélectionnez d’abord un acteur.");
            computed_status.add_css_class("error");
            return;
        };
        match actor_computed_style(&path).and_then(|json| format_computed_style(&json)) {
            Ok(report) => {
                *report_for_refresh.borrow_mut() = report;
                computed_view.buffer().set_text(&filter_computed_style(
                    &report_for_refresh.borrow(),
                    search_for_refresh.text().as_str(),
                ));
                computed_status
                    .set_text("Style calculé actualisé ; filtrez par nom ou valeur de propriété.");
                computed_status.remove_css_class("error");
            }
            Err(error) => {
                computed_status.set_text(&format!("Échec de lecture du style calculé : {error}"));
                computed_status.add_css_class("error");
            }
        }
    });
    computed_actions.append(&computed_refresh);

    let theme_details = gtk::TextView::new();
    theme_details.set_editable(false);
    theme_details.set_cursor_visible(false);
    theme_details.set_monospace(true);
    theme_details.set_wrap_mode(gtk::WrapMode::WordChar);
    theme_details.set_left_margin(12);
    theme_details.set_top_margin(12);
    let theme_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(110)
        .max_content_height(180)
        .child(&theme_details)
        .build();
    let theme_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    theme_actions.set_margin_start(24);
    let theme_refresh = gtk::Button::with_label("Découvrir les feuilles de style");
    let theme_view = theme_details.clone();
    let theme_status = status.clone();
    theme_refresh.connect_clicked(move |_| {
        match theme_info().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(&json)
                .and_then(|value| serde_json::to_string_pretty(&value))
                .map_err(|error| error.to_string())
        }) {
            Ok(info) => {
                theme_view.buffer().set_text(&info);
                theme_status.set_text("Informations du thème actualisées.");
                theme_status.remove_css_class("error");
            }
            Err(error) => {
                theme_status.set_text(&format!("Échec de découverte du thème : {error}"));
                theme_status.add_css_class("error");
            }
        }
    });
    theme_actions.append(&theme_refresh);
    let theme_reload = gtk::Button::with_label("Recharger les feuilles personnalisées");
    let theme_status_for_reload = status.clone();
    theme_reload.connect_clicked(move |_| match reload_custom_stylesheets() {
        Ok(result) if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) => {
            let count = result
                .get("reloaded")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            theme_status_for_reload.set_text(&format!(
                "{count} feuille(s) personnalisée(s) rechargée(s)."
            ));
            theme_status_for_reload.remove_css_class("error");
        }
        Ok(result) => {
            let errors = result
                .get("errors")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let file = item.get("file")?.as_str()?;
                            let message = item.get("message")?.as_str()?;
                            Some(format!("{file}: {message}"))
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                })
                .unwrap_or_else(|| "Erreur inconnue.".to_string());
            theme_status_for_reload.set_text(&format!("Rechargement partiel : {errors}"));
            theme_status_for_reload.add_css_class("error");
        }
        Err(error) => {
            theme_status_for_reload
                .set_text(&format!("Échec du rechargement des feuilles : {error}"));
            theme_status_for_reload.add_css_class("error");
        }
    });
    theme_actions.append(&theme_reload);

    let custom_css = gtk::TextView::new();
    custom_css.set_monospace(true);
    custom_css.set_wrap_mode(gtk::WrapMode::None);
    custom_css.set_vexpand(true);
    custom_css.set_left_margin(8);
    custom_css.set_top_margin(8);
    custom_css.buffer().set_text(
        &custom_stylesheet()
            .unwrap_or_else(|_| "/* CSS personnalisé GNOME Shell */\\n".to_string()),
    );
    let custom_css_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(180)
        .max_content_height(260)
        .hexpand(true)
        .child(&custom_css)
        .build();
    let custom_css_save = gtk::Button::with_label("Enregistrer et appliquer custom.css");
    let custom_css_buffer = custom_css.buffer();
    let custom_css_status = status.clone();
    custom_css_save.connect_clicked(move |_| {
        let start = custom_css_buffer.start_iter();
        let end = custom_css_buffer.end_iter();
        let css = custom_css_buffer.text(&start, &end, false).to_string();
        match save_custom_stylesheet(&css) {
            Ok(result) if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) => {
                let path = result
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("");
                custom_css_status
                    .set_text(&format!("Feuille CSS enregistrée et appliquée : {path}"));
                custom_css_status.remove_css_class("error");
            }
            Ok(result) => {
                let error = result
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Erreur inconnue.");
                custom_css_status.set_text(&format!("Échec de l’application CSS : {error}"));
                custom_css_status.add_css_class("error");
            }
            Err(error) => {
                custom_css_status.set_text(&format!("Échec de l’enregistrement CSS : {error}"));
                custom_css_status.add_css_class("error");
            }
        }
    });
    theme_actions.append(&custom_css_save);

    let properties_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    properties_panel.set_margin_top(8);
    properties_panel.set_margin_bottom(8);
    properties_panel.set_margin_start(12);
    properties_panel.set_margin_end(12);
    let properties_title = gtk::Label::new(Some("Propriétés"));
    properties_title.add_css_class("heading");
    properties_title.set_halign(gtk::Align::Start);
    properties_panel.append(&properties_title);
    properties_panel.append(&details_scroll);

    let inspector = gtk::Box::new(gtk::Orientation::Vertical, 10);
    inspector.set_margin_top(8);
    inspector.set_margin_bottom(8);
    inspector.set_margin_start(12);
    inspector.set_margin_end(12);

    let styles_title = gtk::Label::new(Some("Styles"));
    styles_title.add_css_class("heading");
    styles_title.set_halign(gtk::Align::Start);
    inspector.append(&styles_title);
    inspector.append(&class_row);
    inspector.append(&style_row);

    let computed_title = gtk::Label::new(Some("Style calculé"));
    computed_title.add_css_class("heading");
    computed_title.set_halign(gtk::Align::Start);
    inspector.append(&computed_title);
    inspector.append(&computed_actions);
    inspector.append(&computed_search);
    inspector.append(&computed_scroll);

    let theme_title = gtk::Label::new(Some("Thème"));
    theme_title.add_css_class("heading");
    theme_title.set_halign(gtk::Align::Start);
    inspector.append(&theme_title);
    inspector.append(&theme_actions);
    inspector.append(&custom_css_scroll);
    inspector.append(&theme_scroll);

    let inspector_scroll = gtk::ScrolledWindow::builder()
        .min_content_width(340)
        .hexpand(true)
        .vexpand(true)
        .child(&inspector)
        .build();

    let detail_split = gtk::Paned::new(gtk::Orientation::Horizontal);
    detail_split.set_start_child(Some(&properties_panel));
    detail_split.set_end_child(Some(&inspector_scroll));
    detail_split.set_resize_start_child(true);
    detail_split.set_shrink_start_child(false);
    detail_split.set_position(360);

    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_start_child(Some(&list_scroll));
    split.set_end_child(Some(&detail_split));
    split.set_resize_start_child(false);
    split.set_shrink_start_child(false);
    split.set_position(280);
    split.set_vexpand(true);

    page.append(&actions);
    page.append(&status);
    page.append(&split);
    page
}

fn console_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(
        "Console GJS",
        "Exécutez du JavaScript dans le processus GNOME Shell connecté.",
    ));

    let warning = gtk::Label::new(Some(
        "Attention : le code s’exécute avec les privilèges de votre session GNOME Shell. N’exécutez que du code de confiance.",
    ));
    warning.set_wrap(true);
    warning.set_xalign(0.0);
    warning.set_margin_start(24);
    warning.set_margin_end(24);
    warning.add_css_class("warning");
    page.append(&warning);

    let editor = gtk::TextView::new();
    editor.set_monospace(true);
    editor.set_wrap_mode(gtk::WrapMode::WordChar);
    editor.set_left_margin(12);
    editor.set_top_margin(12);
    editor
        .buffer()
        .set_text("// Exemple : Main.panel.statusArea.dateMenu\nObject.keys(global).slice(0, 10)");
    let editor_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(150)
        .vexpand(true)
        .child(&editor)
        .build();

    let output = gtk::TextView::new();
    output.set_editable(false);
    output.set_cursor_visible(false);
    output.set_monospace(true);
    output.set_wrap_mode(gtk::WrapMode::WordChar);
    output.set_left_margin(12);
    output.set_top_margin(12);
    let output_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(160)
        .vexpand(true)
        .child(&output)
        .build();

    let status = gtk::Label::new(Some("Prêt."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(24);
    status.add_css_class("dim-label");

    let run = gtk::Button::with_label("Exécuter dans GNOME Shell");
    run.add_css_class("suggested-action");
    run.set_halign(gtk::Align::Start);
    run.set_tooltip_text(Some("Exécuter le code · Ctrl+Entrée"));
    let editor_for_run = editor.clone();
    let output_for_run = output.clone();
    let status_for_run = status.clone();
    run.connect_clicked(move |_| {
        let buffer = editor_for_run.buffer();
        let source = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        match evaluate_gjs(source.as_str()) {
            Ok(result) => {
                output_for_run.buffer().set_text(&result);
                status_for_run.set_text("Exécution terminée.");
                status_for_run.remove_css_class("error");
                status_for_run.remove_css_class("dim-label");
                status_for_run.add_css_class("success");
            }
            Err(error) => {
                output_for_run.buffer().set_text(&error);
                status_for_run.set_text("Échec de l’exécution.");
                status_for_run.remove_css_class("success");
                status_for_run.add_css_class("error");
            }
        }
    });

    let clear = gtk::Button::with_label("Effacer le résultat");
    clear.set_sensitive(false);
    let output_for_clear = output.clone();
    clear.connect_clicked(move |_| output_for_clear.buffer().set_text(""));

    let copy = gtk::Button::with_label("Copier le résultat");
    copy.set_sensitive(false);
    let output_for_copy = output.clone();
    copy.connect_clicked(move |_| {
        let buffer = output_for_copy.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        if let Some(display) = gtk::gdk::Display::default() {
            display.clipboard().set_text(text.as_str());
        }
    });

    let run_for_editor = run.clone();
    editor.buffer().connect_changed(move |buffer| {
        let source = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        run_for_editor.set_sensitive(!source.trim().is_empty());
    });

    let clear_for_output = clear.clone();
    let copy_for_output = copy.clone();
    output.buffer().connect_changed(move |buffer| {
        let has_output = !buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .is_empty();
        clear_for_output.set_sensitive(has_output);
        copy_for_output.set_sensitive(has_output);
    });

    let editor_keys = gtk::EventControllerKey::new();
    let run_for_shortcut = run.clone();
    editor_keys.connect_key_pressed(move |_, key, _, modifiers| {
        let control_pressed = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
        if key == gtk::gdk::Key::Return && control_pressed {
            run_for_shortcut.emit_clicked();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    editor.add_controller(editor_keys);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_start(24);
    actions.set_margin_end(24);
    actions.append(&run);
    actions.append(&copy);
    actions.append(&clear);
    page.append(&actions);
    page.append(&editor_scroll);
    page.append(&status);
    page.append(&output_scroll);
    page
}

fn json_report_page(
    title: &str,
    description: &str,
    button_label: &str,
    fetch: fn() -> Result<String, String>,
) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(title, description));

    let status = gtk::Label::new(Some("Aucune collecte effectuée."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(24);
    status.add_css_class("dim-label");

    let view = gtk::TextView::new();
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_left_margin(12);
    view.set_top_margin(12);
    let scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&view)
        .build();

    let refresh = gtk::Button::with_label(button_label);
    refresh.add_css_class("suggested-action");
    refresh.set_halign(gtk::Align::Start);
    let view_for_refresh = view.clone();
    let status_for_refresh = status.clone();
    refresh.connect_clicked(move |_| {
        match fetch().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(&json)
                .and_then(|value| serde_json::to_string_pretty(&value))
                .map_err(|error| error.to_string())
        }) {
            Ok(report) => {
                view_for_refresh.buffer().set_text(&report);
                status_for_refresh.set_text("Collecte actualisée.");
                status_for_refresh.remove_css_class("error");
            }
            Err(error) => {
                status_for_refresh.set_text(&format!("Échec de la collecte : {error}"));
                status_for_refresh.add_css_class("error");
            }
        }
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_start(24);
    actions.set_margin_end(24);
    actions.append(&refresh);
    page.append(&actions);
    page.append(&status);
    page.append(&scroll);
    page
}

fn diagnostics_page() -> gtk::Box {
    json_report_page(
        "Diagnostics",
        "Informations sur la session GNOME Shell, l’affichage et l’agent GNOME Inspector.",
        "Collecter les diagnostics",
        diagnostics,
    )
}

fn performance_metric(title: &str) -> (gtk::Frame, gtk::Label) {
    let value = gtk::Label::new(Some("—"));
    value.add_css_class("title-2");
    value.set_halign(gtk::Align::Start);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    let caption = gtk::Label::new(Some(title));
    caption.add_css_class("dim-label");
    caption.set_halign(gtk::Align::Start);
    content.append(&caption);
    content.append(&value);

    let frame = gtk::Frame::new(None);
    frame.set_child(Some(&content));
    (frame, value)
}

fn performance_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(
        "Performances",
        "Suivez le rendu de GNOME Shell, la taille de la scène et le coût de capture.",
    ));

    let grid = gtk::Grid::new();
    grid.set_column_spacing(12);
    grid.set_row_spacing(12);
    grid.set_margin_start(24);
    grid.set_margin_end(24);

    let (fps_card, fps) = performance_metric("Images rendues / seconde");
    let (frames_card, frames) = performance_metric("Images rendues depuis activation");
    let (duration_card, duration) = performance_metric("Durée de capture");
    let (actors_card, actors) = performance_metric("Acteurs");
    let (stage_card, stage) = performance_metric("Dimensions de la scène");
    let (monitors_card, monitors) = performance_metric("Moniteurs");

    for (index, card) in [
        &fps_card,
        &frames_card,
        &duration_card,
        &actors_card,
        &stage_card,
        &monitors_card,
    ]
    .into_iter()
    .enumerate()
    {
        grid.attach(card, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }

    let status = gtk::Label::new(Some("Aucune mesure effectuée."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(24);
    status.add_css_class("dim-label");

    let history = gtk::TextView::new();
    history.set_editable(false);
    history.set_cursor_visible(false);
    history.set_monospace(true);
    history.set_wrap_mode(gtk::WrapMode::WordChar);
    history.set_left_margin(12);
    history.set_top_margin(12);
    let history_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(180)
        .vexpand(true)
        .hexpand(true)
        .child(&history)
        .build();

    let refresh = gtk::Button::with_label("Actualiser les mesures");
    refresh.add_css_class("suggested-action");
    refresh.set_halign(gtk::Align::Start);
    let fps_value = fps.clone();
    let frames_value = frames.clone();
    let duration_value = duration.clone();
    let actors_value = actors.clone();
    let stage_value = stage.clone();
    let monitors_value = monitors.clone();
    let history_value = history.clone();
    let status_value = status.clone();

    refresh.connect_clicked(move |_| {
        let result = performance().and_then(|json| {
            serde_json::from_str::<serde_json::Value>(&json).map_err(|error| error.to_string())
        });
        match result {
            Ok(report) => {
                let number = |key: &str| report.get(key).and_then(serde_json::Value::as_u64);
                let fps = report
                    .get("framesPerSecond")
                    .and_then(serde_json::Value::as_f64)
                    .map(|value| format!("{value:.1} FPS"))
                    .unwrap_or_else(|| "En attente d’échantillon".to_string());
                fps_value.set_text(&fps);
                frames_value.set_text(
                    &number("renderedFrames")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "—".to_string()),
                );
                duration_value.set_text(
                    &number("captureDurationUs")
                        .map(|value| format!("{:.2} ms", value as f64 / 1000.0))
                        .unwrap_or_else(|| "—".to_string()),
                );
                actors_value.set_text(
                    &number("actorCount")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "—".to_string()),
                );
                let dimensions = report
                    .get("stage")
                    .map(|stage| {
                        format!(
                            "{} × {} px",
                            stage
                                .get("width")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0),
                            stage
                                .get("height")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0),
                        )
                    })
                    .unwrap_or_else(|| "—".to_string());
                stage_value.set_text(&dimensions);
                monitors_value.set_text(
                    &number("monitorCount")
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "—".to_string()),
                );

                let rows = report
                    .get("performanceHistory")
                    .and_then(serde_json::Value::as_array)
                    .map(|samples| {
                        let mut output = String::from("Images rendues     FPS\n");
                        for sample in samples {
                            let count = sample
                                .get("renderedFrames")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0);
                            let rate = sample
                                .get("framesPerSecond")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.0);
                            output.push_str(&format!("{count:>14}  {rate:>6.1}\n"));
                        }
                        output
                    })
                    .unwrap_or_else(|| "Aucun échantillon disponible.".to_string());
                history_value.buffer().set_text(&rows);
                let incomplete = report
                    .get("incomplete")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let truncated = report
                    .get("truncated")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                status_value.set_text(if incomplete || truncated {
                    "Mesure actualisée — capture de scène partielle ou limitée."
                } else {
                    "Mesure actualisée. Le débit d’images est agrégé sur les vues des moniteurs."
                });
                status_value.remove_css_class("error");
            }
            Err(error) => {
                status_value.set_text(&format!("Échec de la mesure : {error}"));
                status_value.add_css_class("error");
            }
        }
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_start(24);
    actions.set_margin_end(24);
    actions.append(&refresh);

    let history_title = gtk::Label::new(Some("Historique récent"));
    history_title.add_css_class("title-3");
    history_title.set_halign(gtk::Align::Start);
    history_title.set_margin_start(24);

    page.append(&grid);
    page.append(&actions);
    page.append(&status);
    page.append(&history_title);
    page.append(&history_scroll);
    page
}

fn overview_page(connection: &gtk::Label) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(
        "Vue d’ensemble",
        "Outils de développement et d’inspection pour GNOME Shell.",
    ));

    let card = gtk::Frame::new(Some("Session GNOME Shell"));
    let details = gtk::Box::new(gtk::Orientation::Vertical, 8);
    details.set_margin_top(16);
    details.set_margin_bottom(16);
    details.set_margin_start(16);
    details.set_margin_end(16);
    details.append(connection);

    let setup = gtk::Box::new(gtk::Orientation::Vertical, 8);
    setup.set_margin_top(8);
    setup.set_visible(false);
    let setup_title = gtk::Label::new(Some("Activation manuelle requise"));
    setup_title.add_css_class("heading");
    setup_title.set_halign(gtk::Align::Start);
    let setup_help = gtk::Label::new(Some(
        "L’installation ou l’activation automatique n’a pas abouti. Vérifiez que GNOME Shell 51 est disponible et que gnome-extensions est installé, puis réessayez la commande ci-dessous.",
    ));
    setup_help.set_halign(gtk::Align::Start);
    setup_help.set_wrap(true);
    setup_help.set_xalign(0.0);
    let install_command = "gnome-extensions enable gnome-inspector@cgreg21";
    let command_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let command = gtk::Label::new(Some(install_command));
    command.set_selectable(true);
    command.set_hexpand(true);
    command.set_halign(gtk::Align::Start);
    command.add_css_class("monospace");
    let copy = gtk::Button::with_label("Copier");
    copy.set_tooltip_text(Some("Copier la commande d’activation"));
    copy.connect_clicked(move |button| {
        if let Some(display) = gtk::gdk::Display::default() {
            display.clipboard().set_text(install_command);
            button.set_label("Copié");
        }
    });
    command_row.append(&command);
    command_row.append(&copy);
    let manual_help = gtk::Label::new(Some(
        "Sous Wayland, GNOME Shell peut nécessiter une reconnexion de session pour charger une extension nouvellement installée. GNOME Inspector ne demande jamais de privilèges administrateur : l’extension est installée uniquement dans votre compte.",
    ));
    manual_help.set_halign(gtk::Align::Start);
    manual_help.set_wrap(true);
    manual_help.set_xalign(0.0);
    manual_help.add_css_class("dim-label");
    setup.append(&setup_title);
    setup.append(&setup_help);
    setup.append(&command_row);
    setup.append(&manual_help);
    refresh_connection(connection, &setup);
    details.append(&setup);

    let retry = gtk::Button::with_label("Tester la connexion");
    retry.set_halign(gtk::Align::Start);
    let connection_for_retry = connection.clone();
    let setup_for_retry = setup.clone();
    retry.connect_clicked(move |_| refresh_connection(&connection_for_retry, &setup_for_retry));
    details.append(&retry);
    card.set_child(Some(&details));
    card.set_margin_start(24);
    card.set_margin_end(24);
    page.append(&card);

    let summary = gtk::Label::new(Some(
        "Modules : Acteurs · CSS · Console GJS · Diagnostics · Performances",
    ));
    summary.set_halign(gtk::Align::Start);
    summary.set_margin_start(24);
    summary.set_margin_top(12);
    summary.set_wrap(true);
    page.append(&summary);
    page
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("GNOME Inspector")
        .default_width(1200)
        .default_height(780)
        .build();

    let connection = gtk::Label::new(Some("Vérification de la connexion…"));
    connection.set_halign(gtk::Align::Start);

    let stack = adw::ViewStack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    stack
        .add_titled(&overview_page(&connection), Some("overview"), "Accueil")
        .set_icon_name(Some("go-home-symbolic"));
    stack
        .add_titled(&actors_page(), Some("actors"), "Acteurs")
        .set_icon_name(Some("view-list-symbolic"));
    stack
        .add_titled(&styles_page(), Some("styles"), "CSS")
        .set_icon_name(Some("applications-graphics-symbolic"));
    stack
        .add_titled(&console_page(), Some("console"), "Console")
        .set_icon_name(Some("utilities-terminal-symbolic"));
    stack
        .add_titled(&diagnostics_page(), Some("diagnostics"), "Diagnostics")
        .set_icon_name(Some("dialog-information-symbolic"));
    stack
        .add_titled(&performance_page(), Some("performance"), "Performances")
        .set_icon_name(Some("utilities-system-monitor-symbolic"));

    let switcher = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&switcher));

    let console_output = gtk::TextView::new();
    console_output.set_editable(false);
    console_output.set_cursor_visible(false);
    console_output.set_monospace(true);
    console_output.set_wrap_mode(gtk::WrapMode::WordChar);
    console_output.set_left_margin(8);
    console_output.set_top_margin(8);
    let console_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(90)
        .max_content_height(180)
        .child(&console_output)
        .build();
    let console_command = gtk::Entry::new();
    console_command.set_placeholder_text(Some("Évaluer une expression GJS dans GNOME Shell…"));
    console_command.set_tooltip_text(Some("Entrée : exécuter · ↑/↓ : parcourir l’historique"));
    console_command.set_hexpand(true);
    let console_run = gtk::Button::with_label("Exécuter");
    console_run.add_css_class("suggested-action");
    console_run.set_sensitive(false);
    let console_history = Rc::new(RefCell::new(Vec::<String>::new()));
    let history_position = Rc::new(RefCell::new(None::<usize>));
    let command_for_run = console_command.clone();
    let output_for_run = console_output.clone();
    let history_for_run = console_history.clone();
    let position_for_run = history_position.clone();
    let scroll_for_run = console_scroll.clone();
    let execute_console: Rc<dyn Fn()> = Rc::new(move || {
        let source = command_for_run.text();
        if source.trim().is_empty() {
            return;
        }
        remember_console_command(&mut history_for_run.borrow_mut(), source.as_str());
        *position_for_run.borrow_mut() = None;
        let result = evaluate_gjs(source.as_str()).unwrap_or_else(|error| error);
        let buffer = output_for_run.buffer();
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, &format_console_entry(&source, &result));
        let adjustment = scroll_for_run.vadjustment();
        adjustment.set_value(adjustment.upper());
        command_for_run.set_text("");
        command_for_run.grab_focus();
    });
    let execute_from_button = execute_console.clone();
    console_run.connect_clicked(move |_| execute_from_button());
    let execute_from_entry = execute_console.clone();
    console_command.connect_activate(move |_| execute_from_entry());
    let run_for_command = console_run.clone();
    console_command.connect_changed(move |entry| {
        run_for_command.set_sensitive(!entry.text().trim().is_empty());
    });

    let history_keys = gtk::EventControllerKey::new();
    let command_for_history = console_command.clone();
    let history_for_keys = console_history.clone();
    let position_for_keys = history_position.clone();
    history_keys.connect_key_pressed(move |_, key, _, _| {
        let history = history_for_keys.borrow();
        if history.is_empty() {
            return glib::Propagation::Proceed;
        }
        match key {
            gtk::gdk::Key::Up => {
                let index = position_for_keys
                    .borrow()
                    .unwrap_or(history.len())
                    .saturating_sub(1);
                *position_for_keys.borrow_mut() = Some(index);
                command_for_history.set_text(&history[index]);
                glib::Propagation::Stop
            }
            gtk::gdk::Key::Down => {
                let position = *position_for_keys.borrow();
                if let Some(index) = position {
                    if index + 1 < history.len() {
                        *position_for_keys.borrow_mut() = Some(index + 1);
                        command_for_history.set_text(&history[index + 1]);
                    } else {
                        *position_for_keys.borrow_mut() = None;
                        command_for_history.set_text("");
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            _ => glib::Propagation::Proceed,
        }
    });
    console_command.add_controller(history_keys);

    let console_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    console_actions.set_margin_start(12);
    console_actions.set_margin_end(12);
    console_actions.set_margin_top(8);
    console_actions.set_margin_bottom(8);
    console_actions.append(&console_command);
    console_actions.append(&console_run);
    let console_clear = gtk::Button::from_icon_name("edit-clear-symbolic");
    console_clear.set_tooltip_text(Some("Effacer la sortie de la console"));
    console_clear.set_sensitive(false);
    let output_for_clear = console_output.clone();
    console_clear.connect_clicked(move |_| output_for_clear.buffer().set_text(""));
    let clear_for_console_output = console_clear.clone();
    console_output.buffer().connect_changed(move |buffer| {
        let has_output = !buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .is_empty();
        clear_for_console_output.set_sensitive(has_output);
    });
    console_actions.append(&console_clear);
    let console_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
    console_panel.append(&console_scroll);
    console_panel.append(&console_actions);
    let console_revealer = gtk::Revealer::builder()
        .transition_type(gtk::RevealerTransitionType::SlideUp)
        .reveal_child(false)
        .child(&console_panel)
        .build();

    let console_toggle = gtk::ToggleButton::with_label("Console");
    console_toggle.set_tooltip_text(Some(
        "Afficher la console GJS persistante (exécute du code dans GNOME Shell)",
    ));
    let console_for_toggle = console_revealer.clone();
    console_toggle.connect_toggled(move |button| {
        console_for_toggle.set_reveal_child(button.is_active());
    });
    header.pack_end(&console_toggle);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&header);
    content.append(&stack);
    content.append(&console_revealer);

    window.set_content(Some(&content));
    window.present();
}

fn main() {
    let app = adw::Application::builder()
        .application_id("io.github.cgreg21.GnomeInspector")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

#[cfg(test)]
mod tests {
    use super::{
        actor_detail_summary, actor_index_by_path, actor_matches, actor_path, actor_selection_index,
        actor_style_label, editable_actor_properties, extension_is_discovered, filter_computed_style,
        flatten_actors, format_actor_details, format_actor_details_filtered, format_computed_style,
        format_console_entry, remember_console_command, string_reply,
    };
    use glib::variant::ToVariant;

    #[test]
    fn detects_only_an_exact_discovered_extension_uuid() {
        let listed = "other@example.com\ngnome-inspector@cgreg21\n";

        assert!(extension_is_discovered(listed, "gnome-inspector@cgreg21"));
        assert!(!extension_is_discovered(listed, "gnome-inspector"));
        assert!(!extension_is_discovered(
            "other@example.com\n",
            "gnome-inspector@cgreg21"
        ));
    }

    #[test]
    fn embeds_all_runtime_extension_files() {
        let metadata: serde_json::Value =
            serde_json::from_str(include_str!("../extension/metadata.json")).unwrap();

        assert_eq!(metadata["uuid"].as_str(), Some("gnome-inspector@cgreg21"));
        assert!(include_str!("../extension/extension.js").contains("GnomeInspectorExtension"));
        assert!(include_str!("../extension/actor-tree.js").contains("buildActorTree"));
        assert!(include_str!("../extension/style-tree.js").contains("buildStyleTree"));
    }

    #[test]
    fn actor_search_keeps_matching_descendant_context() {
        let tree = serde_json::json!({
            "name": "stage",
            "children": [{
                "name": "panel",
                "children": [{
                    "name": "target-button",
                    "type": "StButton",
                    "properties": [{ "name": "label", "value": "Confirm" }],
                    "children": []
                }]
            }]
        });

        assert!(actor_matches(&tree, "target"));
        assert!(actor_matches(&tree["children"][0], "target"));
        assert!(actor_matches(&tree["children"][0]["children"][0], "target"));
        assert!(!actor_matches(&tree, "missing"));
        assert!(actor_matches(&tree, "label"));
        assert!(actor_matches(&tree, "confirm"));
    }

    #[test]
    fn restores_actor_selection_by_stable_path_after_refresh() {
        let nodes = vec![
            serde_json::json!({"path": "0/0"}),
            serde_json::json!({"path": "0/1"}),
            serde_json::json!({"path": "0/2"}),
        ];

        assert_eq!(actor_index_by_path(&nodes, "0/1"), Some(1));
        assert_eq!(actor_index_by_path(&nodes, "0/9"), None);
        assert_eq!(actor_selection_index(&nodes, Some("0/2")), Some(2));
        assert_eq!(actor_selection_index(&nodes, Some("0/9")), Some(0));
        assert_eq!(actor_selection_index(&nodes, None), Some(0));
        assert_eq!(actor_selection_index(&[], None), None);
    }

    #[test]
    fn property_editor_lists_only_writable_supported_properties() {
        let node = serde_json::json!({
            "properties": [
                {"name": "label", "type": "gchararray", "writable": true, "value": "OK"},
                {"name": "visible", "type": "gboolean", "writable": true, "value": true},
                {"name": "width", "type": "gint", "writable": false, "value": 100},
                {"name": "child", "type": "GObject", "writable": true, "value": null}
            ]
        });

        assert_eq!(
            editable_actor_properties(&node),
            vec![
                ("label".to_string(), "OK".to_string()),
                ("visible".to_string(), "true".to_string())
            ]
        );
    }

    #[test]
    fn actor_property_filter_matches_name_type_and_value() {
        let node = serde_json::json!({
            "name": "button",
            "properties": [
                {"name": "label", "type": "gchararray", "readable": true, "value": "Confirm"},
                {"name": "reactive", "type": "gboolean", "readable": true, "value": true}
            ]
        });

        let by_name = format_actor_details_filtered(&node, "label");
        assert!(by_name.contains("label (gchararray"));
        assert!(!by_name.contains("reactive (gboolean"));

        let by_type = format_actor_details_filtered(&node, "gboolean");
        assert!(by_type.contains("reactive (gboolean"));

        let by_value = format_actor_details_filtered(&node, "confirm");
        assert!(by_value.contains("label (gchararray"));
        assert!(!by_value.contains("reactive (gboolean"));
        assert!(by_value.contains("Propriétés (1/2)"));
    }

    #[test]
    fn css_actor_labels_include_stable_paths_for_search() {
        let node = serde_json::json!({
            "name": "button",
            "styleClass": "suggested-action",
            "path": "0/2/1"
        });

        assert_eq!(
            actor_style_label(&node, 2),
            "    button  ·  suggested-action  ·  0/2/1"
        );
    }

    #[test]
    fn reads_actor_path_for_clipboard_actions() {
        let node = serde_json::json!({ "path": "2/1" });
        let missing = serde_json::json!({ "name": "actor" });

        assert_eq!(actor_path(&node), Some("2/1"));
        assert_eq!(actor_path(&missing), None);
    }

    #[test]
    fn actor_details_include_properties_without_descendant_subtrees() {
        let node = serde_json::json!({
            "path": "2/1",
            "name": "target",
            "type": "StButton",
            "visible": true,
            "properties": [{ "name": "label", "type": "gchararray", "readable": true, "writable": false, "value": "OK" }],
            "children": [{ "name": "nested", "children": [] }]
        });

        let details = actor_detail_summary(&node);

        assert_eq!(details["path"], "2/1");
        assert_eq!(details["properties"][0]["value"], "OK");
        assert!(details.get("children").is_none());

        let formatted = format_actor_details(&node);
        assert!(formatted.contains("Propriétés (1)"));
        assert!(formatted.contains("label (gchararray; R) = OK"));
        assert!(!formatted.contains("nested"));
    }

    #[test]
    fn formats_and_filters_computed_style_properties() {
        let report = format_computed_style(r#"{"font":"Sans 12px","width":640}"#).unwrap();
        assert!(report.contains("font: \"Sans 12px\""));
        assert!(report.contains("width: 640"));
        assert_eq!(filter_computed_style(&report, "WIDTH"), "width: 640");
        assert_eq!(
            filter_computed_style(&report, "sans"),
            "font: \"Sans 12px\""
        );
        assert_eq!(filter_computed_style(&report, ""), report);
        assert_eq!(filter_computed_style(&report, "missing"), "");
    }

    #[test]
    fn formats_console_entries_with_real_line_breaks() {
        assert_eq!(format_console_entry("1 + 1", "2"), "> 1 + 1\n2\n\n");
    }

    #[test]
    fn remembers_console_commands_without_consecutive_duplicates() {
        let mut history = Vec::new();
        remember_console_command(&mut history, "1 + 1");
        remember_console_command(&mut history, "1 + 1");
        remember_console_command(&mut history, "2 + 2");

        assert_eq!(history, vec!["1 + 1", "2 + 2"]);
    }

    #[test]
    fn keeps_console_history_bounded_to_fifty_commands() {
        let mut history = Vec::new();
        for command in 0..51 {
            remember_console_command(&mut history, &command.to_string());
        }

        assert_eq!(history.len(), 50);
        assert_eq!(history.first().map(String::as_str), Some("1"));
        assert_eq!(history.last().map(String::as_str), Some("50"));
    }

    #[test]
    fn rejects_non_object_computed_style() {
        assert!(format_computed_style("[]").is_err());
    }

    #[test]
    fn reads_string_from_dbus_reply() {
        let reply = ("51.1",).to_variant();

        assert_eq!(string_reply(&reply, "invalid reply").unwrap(), "51.1");
    }

    #[test]
    fn rejects_dbus_reply_with_wrong_type() {
        let reply = 51u32.to_variant();

        assert_eq!(
            string_reply(&reply, "invalid reply"),
            Err("invalid reply".to_string())
        );
    }

    #[test]
    fn flattens_actor_hierarchy_in_parent_first_order() {
        let tree = serde_json::json!({
            "name": "root",
            "children": [
                {
                    "name": "panel",
                    "children": [
                        { "name": "button", "children": [] }
                    ]
                }
            ]
        });
        let mut rows = Vec::new();

        flatten_actors(&tree, 0, &mut rows);

        let names = rows
            .iter()
            .map(|(node, _)| node["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        let depths = rows.iter().map(|(_, depth)| *depth).collect::<Vec<_>>();

        assert_eq!(names, vec!["root", "panel", "button"]);
        assert_eq!(depths, vec![0, 1, 2]);
    }
}
