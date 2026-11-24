use adw::prelude::*;
use glib::variant::ToVariant;
use std::cell::{Cell, RefCell};
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

fn inspector_section(title: &str, child: &impl IsA<gtk::Widget>, expanded: bool) -> gtk::Expander {
    let section = gtk::Expander::new(Some(title));
    section.set_expanded(expanded);
    section.set_child(Some(child));
    section
}

#[derive(Debug, PartialEq, Eq)]
enum InspectorShortcut {
    ClearSearch,
    PickActor,
    FocusSearch,
    SelectTab(u32),
}

fn inspector_shortcut(
    key: gtk::gdk::Key,
    modifiers: gtk::gdk::ModifierType,
) -> Option<InspectorShortcut> {
    if key == gtk::gdk::Key::Escape {
        return Some(InspectorShortcut::ClearSearch);
    }

    let control = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
    let shift = modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK);
    if control && shift && matches!(key.to_unicode(), Some('c' | 'C')) {
        return Some(InspectorShortcut::PickActor);
    }
    if !control || shift {
        return None;
    }

    match key.to_unicode() {
        Some('1') => Some(InspectorShortcut::SelectTab(0)),
        Some('2') => Some(InspectorShortcut::SelectTab(1)),
        Some('3') => Some(InspectorShortcut::SelectTab(2)),
        Some('f' | 'F') => Some(InspectorShortcut::FocusSearch),
        _ => None,
    }
}

fn text_view_contents(view: &gtk::TextView) -> String {
    let buffer = view.buffer();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string()
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

const LAYOUT_METRICS: [&str; 12] = [
    "marginTop",
    "marginRight",
    "marginBottom",
    "marginLeft",
    "borderTop",
    "borderRight",
    "borderBottom",
    "borderLeft",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
];

fn layout_metric(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .filter(|number| number.is_finite())
        .map(|number| format!("{number} px"))
        .unwrap_or_else(|| "—".to_string())
}

fn computed_style_entries(report: &str, query: &str) -> Vec<(String, String)> {
    let query = query.trim().to_lowercase();
    report
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            let name = name.trim();
            let value = value.trim();
            (!name.is_empty()
                && (query.is_empty()
                    || name.to_lowercase().contains(&query)
                    || value.to_lowercase().contains(&query)))
            .then(|| (name.to_string(), value.to_string()))
        })
        .collect()
}

fn populate_computed_style(list: &gtk::ListBox, report: &str, query: &str) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    for (name, value) in computed_style_entries(report, query) {
        let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_content.set_margin_top(5);
        row_content.set_margin_bottom(5);
        row_content.set_margin_start(10);
        row_content.set_margin_end(8);

        let property = gtk::Label::new(Some(&name));
        property.add_css_class("monospace");
        property.add_css_class("accent");
        property.set_halign(gtk::Align::Start);
        property.set_xalign(0.0);
        property.set_width_chars(20);
        property.set_max_width_chars(28);
        property.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let value_label = gtk::Label::new(Some(&value));
        value_label.add_css_class("monospace");
        value_label.set_halign(gtk::Align::Start);
        value_label.set_xalign(0.0);
        value_label.set_hexpand(true);
        value_label.set_selectable(true);
        value_label.set_wrap(true);

        let copy = gtk::Button::from_icon_name("edit-copy-symbolic");
        copy.set_has_frame(false);
        copy.set_tooltip_text(Some("Copier la valeur calculée"));
        copy.connect_clicked(move |_| {
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&value);
            }
        });

        row_content.append(&property);
        row_content.append(&value_label);
        row_content.append(&copy);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&row_content));
        list.append(&row);
    }

    if list.first_child().is_none() {
        let empty = gtk::Label::new(Some(if query.trim().is_empty() {
            "Aucune propriété calculée disponible."
        } else {
            "Aucune propriété ne correspond au filtre."
        }));
        empty.set_margin_top(16);
        empty.set_margin_bottom(16);
        empty.add_css_class("dim-label");
        list.set_placeholder(Some(&empty));
    }
}

fn refresh_computed_style(
    path: &str,
    list: &gtk::ListBox,
    report: &Rc<RefCell<String>>,
    search: &gtk::SearchEntry,
    layout_values: &[gtk::Label; 12],
    layout_size: &gtk::Label,
) -> Result<(), String> {
    let json = actor_computed_style(path)?;
    let value: serde_json::Value =
        serde_json::from_str(&json).map_err(|error| error.to_string())?;
    let formatted = format_computed_style(&json)?;

    for (label, key) in layout_values.iter().zip(LAYOUT_METRICS) {
        label.set_text(&layout_metric(&value, key));
    }
    layout_size.set_text(&format!(
        "{} × {}",
        layout_metric(&value, "width"),
        layout_metric(&value, "height")
    ));
    *report.borrow_mut() = formatted;
    populate_computed_style(list, &report.borrow(), search.text().as_str());
    Ok(())
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

fn actor_property_has_changed(node: &serde_json::Value, name: &str, value: &str) -> bool {
    editable_actor_properties(node)
        .into_iter()
        .find(|(property, _)| property == name)
        .is_some_and(|(_, original)| original != value)
}

fn text_differs(original: &str, value: &str) -> bool {
    original != value
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

fn actor_picker_button(
    list: &gtk::ListBox,
    nodes: &Rc<RefCell<Vec<serde_json::Value>>>,
    status: &gtk::Label,
) -> gtk::Button {
    let button = gtk::Button::with_label("Sélectionner dans Shell");
    button.set_tooltip_text(Some(
        "Sélectionner un acteur visuellement dans GNOME Shell (Ctrl+Maj+C)",
    ));
    let list = list.clone();
    let nodes = nodes.clone();
    let status = status.clone();

    button.connect_clicked(move |_| match start_actor_picker() {
        Ok(()) => {
            status.set_text("Cliquez sur un acteur dans GNOME Shell · Échap pour annuler.");
            let list = list.clone();
            let nodes = nodes.clone();
            let status = status.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(250), move || {
                match actor_picker_result() {
                    Ok(result) => match result
                        .get("status")
                        .and_then(serde_json::Value::as_str)
                    {
                        Some("pending") => glib::ControlFlow::Continue,
                        Some("selected") => {
                            let path = result
                                .get("path")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("");
                            let index = nodes.borrow().iter().position(|node| {
                                node.get("path").and_then(serde_json::Value::as_str) == Some(path)
                            });
                            if let Some(index) = index
                                && let Some(row) = list.row_at_index(index as i32)
                            {
                                list.select_row(Some(&row));
                                status.set_text(&format!("Acteur sélectionné · chemin {path}"));
                            } else {
                                status.set_text(
                                    "Acteur sélectionné dans Shell ; actualisez l’arbre pour synchroniser la ligne.",
                                );
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
            status.set_text(&format!("Impossible de démarrer la sélection : {error}"));
            status.add_css_class("error");
        }
    });

    button
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

fn actor_breadcrumb(nodes: &[serde_json::Value], path: &str) -> String {
    let mut prefixes = vec![String::new()];
    let mut current = String::new();
    for index in path.split('/').filter(|part| !part.is_empty()) {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(index);
        prefixes.push(current.clone());
    }

    prefixes
        .iter()
        .filter_map(|prefix| {
            nodes
                .iter()
                .find(|node| actor_path(node) == Some(prefix.as_str()))
                .and_then(|node| node.get("name").and_then(serde_json::Value::as_str))
        })
        .collect::<Vec<_>>()
        .join("  ›  ")
}

fn actor_inspector_title(node: &serde_json::Value) -> String {
    let name = node
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Acteur");
    let kind = node
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Type inconnu");
    format!("{name}  ·  {kind}")
}

fn actor_style_summary(node: &serde_json::Value) -> String {
    let classes = node
        .get("styleClass")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .split_whitespace()
        .map(|class| format!(".{class}"))
        .collect::<Vec<_>>();
    let pseudo_classes = node
        .get("pseudoClass")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .split_whitespace()
        .map(|pseudo| format!(":{pseudo}"))
        .collect::<Vec<_>>();
    let summary = classes
        .into_iter()
        .chain(pseudo_classes)
        .collect::<Vec<_>>()
        .join(" ");

    if summary.is_empty() {
        "Aucune classe ni pseudo-classe".to_string()
    } else {
        summary
    }
}

fn actor_path(node: &serde_json::Value) -> Option<&str> {
    node.get("path").and_then(serde_json::Value::as_str)
}

fn actor_index_by_path(nodes: &[serde_json::Value], path: &str) -> Option<usize> {
    nodes.iter().position(|node| actor_path(node) == Some(path))
}

fn actor_parent_path(path: &str) -> Option<&str> {
    if path.is_empty() {
        None
    } else {
        Some(path.rsplit_once('/').map_or("", |(parent, _)| parent))
    }
}

fn actor_first_child_index(nodes: &[serde_json::Value], path: &str) -> Option<usize> {
    nodes
        .iter()
        .position(|node| actor_path(node).and_then(actor_parent_path) == Some(path))
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
    status.set_margin_start(8);
    status.set_margin_end(8);
    status.add_css_class("dim-label");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("navigation-sidebar");
    let empty_state = gtk::Label::new(Some(
        "Aucun acteur affiché. Actualisez la capture ou modifiez votre recherche.",
    ));
    empty_state.set_wrap(true);
    empty_state.set_margin_top(24);
    empty_state.set_margin_bottom(24);
    empty_state.set_margin_start(16);
    empty_state.set_margin_end(16);
    empty_state.add_css_class("dim-label");
    list.set_placeholder(Some(&empty_state));
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
            apply_for_selection.set_sensitive(false);
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
    let apply_for_property_choice = apply_property.clone();
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
        apply_for_property_choice.set_sensitive(false);
    });

    let actor_for_value_change = selected_actor.clone();
    let name_for_value_change = property_name.clone();
    let apply_for_value_change = apply_property.clone();
    property_value.connect_changed(move |entry| {
        let changed = actor_for_value_change
            .borrow()
            .as_ref()
            .zip(name_for_value_change.active_id().as_ref())
            .is_some_and(|(node, name)| {
                actor_property_has_changed(node, name.as_str(), entry.text().as_str())
            });
        apply_for_value_change.set_sensitive(changed);
    });

    let actor_for_apply = selected_actor.clone();
    let name_for_apply = property_name.clone();
    let value_for_apply = property_value.clone();
    let status_for_apply = status.clone();
    let details_for_apply = details.clone();
    let search_for_apply = property_search.clone();
    let apply_property_for_success = apply_property.clone();
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
                apply_property_for_success.set_sensitive(false);
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

    let pick_actor = actor_picker_button(&list, &selected_nodes, &status);

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
    actions.set_margin_start(8);
    actions.set_margin_end(8);
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

fn update_inline_declaration(css: &str, name: &str, value: &str, enabled: bool) -> String {
    let mut declarations = css
        .split(';')
        .filter_map(|declaration| {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                return None;
            }
            match declaration.split_once(':') {
                Some((property, _)) if property.trim() == name => None,
                _ => Some(declaration.to_string()),
            }
        })
        .collect::<Vec<_>>();

    if enabled {
        declarations.push(format!("{name}: {}", value.trim()));
    }

    if declarations.is_empty() {
        String::new()
    } else {
        format!("{};", declarations.join("; "))
    }
}

fn populate_inline_declarations(
    list: &gtk::ListBox,
    css: &str,
    query: &str,
    editor: &gtk::TextView,
    updating: &Rc<Cell<bool>>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let query = query.trim().to_lowercase();
    let mut count = 0;
    for declaration in css.split(';') {
        let Some((name, value)) = declaration.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let value = value.trim();
        if name.is_empty()
            || value.is_empty()
            || (!query.is_empty()
                && !name.to_lowercase().contains(&query)
                && !value.to_lowercase().contains(&query))
        {
            continue;
        }

        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        content.set_margin_top(4);
        content.set_margin_bottom(4);
        content.set_margin_start(6);
        content.set_margin_end(6);

        let enabled = gtk::CheckButton::new();
        enabled.set_active(true);
        enabled.set_tooltip_text(Some("Activer ou désactiver cette déclaration"));
        let property = gtk::Label::new(Some(name));
        property.add_css_class("monospace");
        property.add_css_class("accent");
        property.set_halign(gtk::Align::Start);
        property.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let value_entry = gtk::Entry::new();
        value_entry.set_text(value);
        value_entry.set_has_frame(false);
        value_entry.add_css_class("monospace");
        value_entry.set_hexpand(true);
        value_entry.set_tooltip_text(Some("Modifier la valeur puis appuyer sur Entrée"));

        let property_name = name.to_string();
        let editor_for_toggle = editor.clone();
        let updating_for_toggle = updating.clone();
        let value_for_toggle = value_entry.clone();
        let property_for_toggle = property.clone();
        enabled.connect_toggled(move |button| {
            value_for_toggle.set_sensitive(button.is_active());
            if button.is_active() {
                property_for_toggle.remove_css_class("dim-label");
            } else {
                property_for_toggle.add_css_class("dim-label");
            }
            let css = text_view_contents(&editor_for_toggle);
            let updated = update_inline_declaration(
                &css,
                &property_name,
                value_for_toggle.text().as_str(),
                button.is_active(),
            );
            updating_for_toggle.set(true);
            editor_for_toggle.buffer().set_text(&updated);
            updating_for_toggle.set(false);
        });

        let property_name = name.to_string();
        let editor_for_value = editor.clone();
        let updating_for_value = updating.clone();
        let enabled_for_value = enabled.clone();
        value_entry.connect_activate(move |entry| {
            let css = text_view_contents(&editor_for_value);
            let updated = update_inline_declaration(
                &css,
                &property_name,
                entry.text().as_str(),
                enabled_for_value.is_active(),
            );
            updating_for_value.set(true);
            editor_for_value.buffer().set_text(&updated);
            updating_for_value.set(false);
        });

        let property_name_for_copy = name.to_string();
        let value_for_copy = value_entry.clone();
        let copy_declaration = gtk::Button::from_icon_name("edit-copy-symbolic");
        copy_declaration.set_has_frame(false);
        copy_declaration.set_tooltip_text(Some("Copier cette déclaration CSS"));
        copy_declaration.connect_clicked(move |_| {
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&format!(
                    "{}: {};",
                    property_name_for_copy,
                    value_for_copy.text()
                ));
            }
        });

        content.append(&enabled);
        content.append(&property);
        content.append(&value_entry);
        content.append(&copy_declaration);
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&content));
        list.append(&row);
        count += 1;
    }

    if count == 0 {
        let message = if query.is_empty() {
            "Aucune déclaration inline pour cet acteur."
        } else {
            "Aucune déclaration ne correspond au filtre."
        };
        let empty = gtk::Label::new(Some(message));
        empty.set_margin_top(12);
        empty.set_margin_bottom(12);
        empty.add_css_class("dim-label");
        list.set_placeholder(Some(&empty));
    }
}

fn styles_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let status = gtk::Label::new(Some("Aucune capture CSS effectuée."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(8);
    status.set_margin_end(8);
    status.set_margin_bottom(4);
    status.add_css_class("dim-label");

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("navigation-sidebar");
    let empty_state = gtk::Label::new(Some(
        "Aucun acteur CSS affiché. Actualisez les styles pour commencer.",
    ));
    empty_state.set_wrap(true);
    empty_state.set_margin_top(24);
    empty_state.set_margin_bottom(24);
    empty_state.set_margin_start(16);
    empty_state.set_margin_end(16);
    empty_state.add_css_class("dim-label");
    list.set_placeholder(Some(&empty_state));
    let list_scroll = gtk::ScrolledWindow::builder()
        .min_content_width(280)
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
    let selected_actor_title = gtk::Label::new(Some("Aucun élément sélectionné"));
    selected_actor_title.add_css_class("heading");
    selected_actor_title.set_halign(gtk::Align::Start);
    let selected_actor_style = gtk::Label::new(Some("Aucune classe ni pseudo-classe"));
    selected_actor_style.add_css_class("dim-label");
    selected_actor_style.add_css_class("monospace");
    selected_actor_style.set_halign(gtk::Align::Start);
    selected_actor_style.set_selectable(true);
    let selected_actor_path = gtk::Label::new(Some("Sélectionnez un acteur dans l’arborescence"));
    selected_actor_path.add_css_class("dim-label");
    selected_actor_path.add_css_class("monospace");
    selected_actor_path.set_halign(gtk::Align::Start);
    selected_actor_path.set_selectable(true);
    selected_actor_path.set_ellipsize(gtk::pango::EllipsizeMode::Middle);

    let tree_breadcrumb = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tree_breadcrumb.set_halign(gtk::Align::Start);
    tree_breadcrumb.set_margin_start(10);
    tree_breadcrumb.set_margin_end(10);
    tree_breadcrumb.set_margin_top(6);
    tree_breadcrumb.set_margin_bottom(6);
    let breadcrumb_hint = gtk::Label::new(Some("Sélectionnez un élément dans l’arborescence"));
    breadcrumb_hint.add_css_class("dim-label");
    tree_breadcrumb.append(&breadcrumb_hint);

    let selected_nodes = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
    let query = Rc::new(RefCell::new(String::new()));
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

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Rechercher un acteur ou une classe CSS…"));
    search.set_hexpand(true);
    let query_for_search = query.clone();
    let list_for_search = list.clone();
    let expanders_for_search = Rc::new(RefCell::new(Vec::<(usize, gtk::ToggleButton)>::new()));
    let search_expanders = expanders_for_search.clone();
    search.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        *query_for_search.borrow_mut() = query.clone();
        for (index, expander) in search_expanders.borrow().iter() {
            let is_expanded = !query.is_empty() || *index == 0;
            expander.set_active(is_expanded);
        }
        list_for_search.invalidate_filter();
    });

    let tree_keys = gtk::EventControllerKey::new();
    let tree_for_keys = list.clone();
    let nodes_for_keys = selected_nodes.clone();
    let expanders_for_keys = expanders_for_search.clone();
    let search_for_keys = search.clone();
    tree_keys.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gtk::gdk::Key::f && modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            search_for_keys.grab_focus();
            return glib::Propagation::Stop;
        }
        if key == gtk::gdk::Key::Escape && !search_for_keys.text().is_empty() {
            search_for_keys.set_text("");
            return glib::Propagation::Stop;
        }

        let Some(row) = tree_for_keys.selected_row() else {
            return glib::Propagation::Proceed;
        };
        let Ok(index) = row.widget_name().parse::<usize>() else {
            return glib::Propagation::Proceed;
        };
        let nodes = nodes_for_keys.borrow();
        let Some(path) = nodes.get(index).and_then(actor_path) else {
            return glib::Propagation::Proceed;
        };
        let expander = expanders_for_keys
            .borrow()
            .iter()
            .find(|(row_index, _)| *row_index == index)
            .map(|(_, expander)| expander.clone());

        match key {
            gtk::gdk::Key::Right => {
                if let Some(expander) = expander {
                    if !expander.is_active() {
                        expander.set_active(true);
                    } else if let Some(child_index) = actor_first_child_index(&nodes, path)
                        && let Some(child) = tree_for_keys.row_at_index(child_index as i32)
                    {
                        tree_for_keys.select_row(Some(&child));
                    }
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            gtk::gdk::Key::Left => {
                if let Some(expander) = expander
                    && expander.is_active()
                {
                    expander.set_active(false);
                    glib::Propagation::Stop
                } else if let Some(parent_path) = actor_parent_path(path)
                    && let Some(parent_index) = actor_index_by_path(&nodes, parent_path)
                    && let Some(parent) = tree_for_keys.row_at_index(parent_index as i32)
                {
                    tree_for_keys.select_row(Some(&parent));
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            _ => glib::Propagation::Proceed,
        }
    });
    list.add_controller(tree_keys);

    let selected_path = Rc::new(RefCell::new(None::<String>));
    let class_input = gtk::Entry::new();
    class_input.set_placeholder_text(Some("Classes CSS : button primary"));
    class_input.set_hexpand(true);
    class_input.set_sensitive(false);
    let style_input = gtk::TextView::new();
    style_input.set_monospace(true);
    style_input.set_wrap_mode(gtk::WrapMode::None);
    style_input.set_left_margin(8);
    style_input.set_top_margin(8);
    style_input.set_sensitive(false);
    style_input.add_css_class("card");
    let declaration_filter = gtk::SearchEntry::new();
    declaration_filter.set_placeholder_text(Some("Filtrer les déclarations inline…"));
    declaration_filter.set_sensitive(false);
    let declaration_list = gtk::ListBox::new();
    declaration_list.set_selection_mode(gtk::SelectionMode::None);
    declaration_list.add_css_class("boxed-list");
    let declaration_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(90)
        .max_content_height(180)
        .child(&declaration_list)
        .build();
    let selected_inline_css = Rc::new(RefCell::new(String::new()));
    let updating_declaration = Rc::new(Cell::new(false));

    let style_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(96)
        .max_content_height(160)
        .hexpand(true)
        .child(&style_input)
        .build();

    let nodes_for_selection = selected_nodes.clone();
    let path_for_selection = selected_path.clone();
    let details_for_selection = details.clone();
    let class_for_selection = class_input.clone();
    let style_for_selection = style_input.clone();
    let declarations_for_selection = declaration_list.clone();
    let selected_inline_css_for_selection = selected_inline_css.clone();
    let declaration_filter_for_selection = declaration_filter.clone();
    let updating_for_selection = updating_declaration.clone();
    let title_for_selection = selected_actor_title.clone();
    let style_summary_for_selection = selected_actor_style.clone();
    let path_label_for_selection = selected_actor_path.clone();
    let breadcrumb_for_selection = tree_breadcrumb.clone();
    let tree_for_breadcrumb = list.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row
            && let Ok(index) = row.widget_name().parse::<usize>()
        {
            let nodes = nodes_for_selection.borrow();
            if let Some(node) = nodes.get(index) {
                show_actor_details(&details_for_selection, node);
                title_for_selection.set_text(&actor_inspector_title(node));
                style_summary_for_selection.set_text(&actor_style_summary(node));
                let path = actor_path(node).unwrap_or("").to_string();
                path_label_for_selection.set_text(&path);
                path_label_for_selection.set_tooltip_text(Some(&actor_breadcrumb(&nodes, &path)));
                *path_for_selection.borrow_mut() = Some(path.clone());
                class_for_selection.set_text(
                    node.get("styleClass")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(""),
                );
                let inline_css = node
                    .get("inlineStyle")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string();
                style_for_selection.buffer().set_text(&inline_css);
                *selected_inline_css_for_selection.borrow_mut() = inline_css.clone();
                populate_inline_declarations(
                    &declarations_for_selection,
                    &inline_css,
                    declaration_filter_for_selection.text().as_str(),
                    &style_for_selection,
                    &updating_for_selection,
                );
                declaration_filter_for_selection.set_sensitive(true);

                while let Some(child) = breadcrumb_for_selection.first_child() {
                    breadcrumb_for_selection.remove(&child);
                }
                let mut prefixes = vec![String::new()];
                let mut current = String::new();
                for part in path.split('/').filter(|part| !part.is_empty()) {
                    if !current.is_empty() {
                        current.push('/');
                    }
                    current.push_str(part);
                    prefixes.push(current.clone());
                }
                let crumbs = prefixes
                    .iter()
                    .filter_map(|prefix| {
                        let crumb_index = actor_index_by_path(&nodes, prefix)?;
                        let crumb_node = nodes.get(crumb_index)?;
                        let name = crumb_node
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Acteur");
                        Some((crumb_index, name.to_string(), prefix.clone()))
                    })
                    .collect::<Vec<_>>();
                for (position, (crumb_index, name, crumb_path)) in crumbs.iter().enumerate() {
                    if position > 0 {
                        let separator = gtk::Label::new(Some("›"));
                        separator.add_css_class("dim-label");
                        breadcrumb_for_selection.append(&separator);
                    }
                    let crumb = gtk::Button::with_label(name);
                    crumb.set_has_frame(false);
                    crumb.add_css_class("flat");
                    crumb.add_css_class("monospace");
                    crumb.set_tooltip_text(Some(crumb_path));
                    let tree = tree_for_breadcrumb.clone();
                    let index = *crumb_index;
                    crumb.connect_clicked(move |_| {
                        if let Some(target) = tree.row_at_index(index as i32) {
                            tree.select_row(Some(&target));
                        }
                    });
                    breadcrumb_for_selection.append(&crumb);
                }
            }
        } else {
            title_for_selection.set_text("Aucun élément sélectionné");
            style_summary_for_selection.set_text("Aucune classe ni pseudo-classe");
            path_label_for_selection.set_text("Sélectionnez un acteur dans l’arborescence");
            path_label_for_selection.set_tooltip_text(None);
            *path_for_selection.borrow_mut() = None;
            details_for_selection
                .buffer()
                .set_text("Sélectionnez un acteur pour inspecter ses styles.");
            class_for_selection.set_text("");
            style_for_selection.buffer().set_text("");
            selected_inline_css_for_selection.borrow_mut().clear();
            declaration_filter_for_selection.set_sensitive(false);
            populate_inline_declarations(
                &declarations_for_selection,
                "",
                "",
                &style_for_selection,
                &updating_for_selection,
            );
            while let Some(child) = breadcrumb_for_selection.first_child() {
                breadcrumb_for_selection.remove(&child);
            }
            let hint = gtk::Label::new(Some("Sélectionnez un élément dans l’arborescence"));
            hint.add_css_class("dim-label");
            breadcrumb_for_selection.append(&hint);
        }
    });

    let list_for_refresh = list.clone();
    let details_for_refresh = details.clone();
    let nodes_for_refresh = selected_nodes.clone();
    let path_for_refresh = selected_path.clone();
    let class_for_refresh = class_input.clone();
    let style_for_refresh = style_input.clone();
    let declarations_for_refresh = declaration_list.clone();
    let selected_inline_css_for_refresh = selected_inline_css.clone();
    let declaration_filter_for_refresh = declaration_filter.clone();
    let updating_for_refresh = updating_declaration.clone();
    let status_for_refresh = status.clone();
    let expand_all = gtk::Button::with_label("Tout déplier");
    expand_all.set_tooltip_text(Some("Déplier tous les niveaux de l’arborescence"));
    expand_all.set_sensitive(false);
    let collapse_all = gtk::Button::with_label("Tout replier");
    collapse_all.set_tooltip_text(Some("Replier tous les niveaux de l’arborescence"));
    collapse_all.set_sensitive(false);
    let expand_all_for_refresh = expand_all.clone();
    let collapse_all_for_refresh = collapse_all.clone();
    let expanders_for_expand_all = expanders_for_search.clone();
    expand_all.connect_clicked(move |_| {
        for (_, expander) in expanders_for_expand_all.borrow().iter() {
            expander.set_active(true);
        }
    });
    let expanders_for_collapse_all = expanders_for_search.clone();
    collapse_all.connect_clicked(move |_| {
        for (_, expander) in expanders_for_collapse_all.borrow().iter() {
            expander.set_active(false);
        }
    });

    let refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh.set_tooltip_text(Some("Actualiser l’arborescence et les styles"));
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
                expanders_for_search.borrow_mut().clear();
                let row_widgets = Rc::new(RefCell::new(Vec::<gtk::ListBoxRow>::new()));

                for (index, (node, depth)) in rows.into_iter().enumerate() {
                    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    content.set_margin_top(2);
                    content.set_margin_bottom(2);
                    content.set_margin_start(4 + depth as i32 * 14);
                    content.set_margin_end(8);

                    let has_children = node
                        .get("children")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|children| !children.is_empty());
                    if has_children {
                        let expander = gtk::ToggleButton::with_label("▸");
                        expander.set_active(depth == 0);
                        expander.set_label(if depth == 0 { "▾" } else { "▸" });
                        expander.set_has_frame(false);
                        expander.set_size_request(22, 24);
                        expander.set_tooltip_text(Some("Déplier ou replier les éléments enfants"));
                        expanders_for_search
                            .borrow_mut()
                            .push((index, expander.clone()));

                        let expanded_for_toggle = expanded.clone();
                        let parents_for_toggle = parents.clone();
                        let rows_for_toggle = row_widgets.clone();
                        expander.connect_toggled(move |button| {
                            let is_expanded = button.is_active();
                            button.set_label(if is_expanded { "▾" } else { "▸" });
                            if let Some(state) = expanded_for_toggle.borrow_mut().get_mut(index) {
                                *state = is_expanded;
                            }
                            let states = expanded_for_toggle.borrow();
                            for (row_index, row) in rows_for_toggle.borrow().iter().enumerate() {
                                let mut parent = parents_for_toggle[row_index];
                                let mut visible = true;
                                while let Some(parent_index) = parent {
                                    if !states[parent_index] {
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
                        spacer.set_size_request(22, -1);
                        content.append(&spacer);
                    }

                    let name = node
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Acteur");
                    let name_label = gtk::Label::new(Some(name));
                    name_label.set_halign(gtk::Align::Start);
                    name_label.set_hexpand(true);
                    name_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    content.append(&name_label);

                    let kind = node
                        .get("type")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Actor");
                    let type_label = gtk::Label::new(Some(kind));
                    type_label.add_css_class("dim-label");
                    type_label.add_css_class("caption");
                    type_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    content.append(&type_label);

                    let classes = node
                        .get("styleClass")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    if !classes.is_empty() {
                        let classes = classes
                            .split_whitespace()
                            .map(|class| format!(".{class}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let class_label = gtk::Label::new(Some(&classes));
                        class_label.add_css_class("dim-label");
                        class_label.add_css_class("monospace");
                        class_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                        class_label.set_max_width_chars(24);
                        content.append(&class_label);
                    }

                    let pseudo_class = node
                        .get("pseudoClass")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    if !pseudo_class.is_empty() {
                        let pseudo_label = gtk::Label::new(Some(&format!(":{pseudo_class}")));
                        pseudo_label.add_css_class("dim-label");
                        pseudo_label.add_css_class("monospace");
                        pseudo_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                        content.append(&pseudo_label);
                    }

                    let row = gtk::ListBoxRow::new();
                    row.set_widget_name(&index.to_string());
                    row.set_tooltip_text(actor_path(&node));
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
                    list_for_refresh.append(&row);
                    row_widgets.borrow_mut().push(row);
                }

                let has_expandable_nodes = !expanders_for_search.borrow().is_empty();
                expand_all_for_refresh.set_sensitive(has_expandable_nodes);
                collapse_all_for_refresh.set_sensitive(has_expandable_nodes);

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
                    style_for_refresh.buffer().set_text("");
                    *selected_inline_css_for_refresh.borrow_mut() = String::new();
                    populate_inline_declarations(
                        &declarations_for_refresh,
                        "",
                        declaration_filter_for_refresh.text().as_str(),
                        &style_for_refresh,
                        &updating_for_refresh,
                    );
                    declaration_filter_for_refresh.set_sensitive(false);
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
    actions.set_margin_start(8);
    actions.set_margin_end(8);
    actions.set_margin_top(6);
    actions.add_css_class("toolbar");
    let pick_actor = actor_picker_button(&list, &selected_nodes, &status);
    actions.append(&search);
    actions.append(&pick_actor);
    actions.append(&expand_all);
    actions.append(&collapse_all);
    actions.append(&refresh);

    let class_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    class_row.set_margin_start(0);
    class_row.set_margin_end(0);
    let original_class = Rc::new(RefCell::new(String::new()));
    let original_style = Rc::new(RefCell::new(String::new()));

    let apply_class = gtk::Button::with_label("Appliquer les classes");
    apply_class.set_sensitive(false);
    let class_for_apply = class_input.clone();
    let path_for_class = selected_path.clone();
    let status_for_class = status.clone();
    let original_class_for_apply = original_class.clone();
    let apply_class_for_success = apply_class.clone();
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
                    *original_class_for_apply.borrow_mut() = class_for_apply.text().to_string();
                    apply_class_for_success.set_sensitive(false);
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
    style_row.set_margin_start(0);
    style_row.set_margin_end(0);
    let apply_style = gtk::Button::with_label("Appliquer le style inline");
    apply_style.set_sensitive(false);
    let style_for_apply = style_input.clone();
    let path_for_style = selected_path.clone();
    let status_for_style = status.clone();
    let original_style_for_apply = original_style.clone();
    let apply_style_for_success = apply_style.clone();
    apply_style.connect_clicked(move |_| {
        let Some(path) = path_for_style.borrow().clone() else {
            status_for_style.set_text("Sélectionnez d’abord un acteur.");
            status_for_style.add_css_class("error");
            return;
        };
        let css = text_view_contents(&style_for_apply);
        match set_actor_inline_style(&path, &css) {
            Ok(reply) => match serde_json::from_str::<serde_json::Value>(&reply) {
                Ok(result)
                    if result.get("ok").and_then(serde_json::Value::as_bool) == Some(true) =>
                {
                    *original_style_for_apply.borrow_mut() = text_view_contents(&style_for_apply);
                    apply_style_for_success.set_sensitive(false);
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
    let style_editor = gtk::Box::new(gtk::Orientation::Vertical, 6);
    style_editor.append(&style_scroll);
    style_editor.append(&apply_style);
    let style_keys = gtk::EventControllerKey::new();
    let apply_style_for_shortcut = apply_style.clone();
    style_keys.connect_key_pressed(move |_, key, _, modifiers| {
        if key == gtk::gdk::Key::Return && modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
        {
            apply_style_for_shortcut.emit_clicked();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    style_input.add_controller(style_keys);
    style_row.append(&style_editor);

    let original_class_for_change = original_class.clone();
    let apply_class_for_change = apply_class.clone();
    class_input.connect_changed(move |entry| {
        apply_class_for_change.set_sensitive(text_differs(
            &original_class_for_change.borrow(),
            entry.text().as_str(),
        ));
    });

    let original_style_for_change = original_style.clone();
    let apply_style_for_change = apply_style.clone();
    let declaration_list_for_edit = declaration_list.clone();
    let selected_css_for_edit = selected_inline_css.clone();
    let declaration_filter_for_edit = declaration_filter.clone();
    let style_for_edit = style_input.clone();
    let updating_for_edit = updating_declaration.clone();
    style_input.buffer().connect_changed(move |buffer| {
        let css = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();
        *selected_css_for_edit.borrow_mut() = css.clone();
        if !updating_for_edit.get() {
            populate_inline_declarations(
                &declaration_list_for_edit,
                &css,
                declaration_filter_for_edit.text().as_str(),
                &style_for_edit,
                &updating_for_edit,
            );
        }
    });

    style_input.buffer().connect_changed(move |buffer| {
        let value = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        apply_style_for_change.set_sensitive(text_differs(
            &original_style_for_change.borrow(),
            value.as_str(),
        ));
    });

    let list_for_edit_state = list.clone();
    let class_for_edit_state = class_input.clone();
    let style_for_edit_state = style_input.clone();
    let original_class_for_selection = original_class.clone();
    let original_style_for_selection = original_style.clone();
    let apply_class_for_edit_state = apply_class.clone();
    let apply_style_for_edit_state = apply_style.clone();
    list_for_edit_state.connect_row_selected(move |_, row| {
        let enabled = row.is_some();
        class_for_edit_state.set_sensitive(enabled);
        style_for_edit_state.set_sensitive(enabled);
        if enabled {
            *original_class_for_selection.borrow_mut() = class_for_edit_state.text().to_string();
            *original_style_for_selection.borrow_mut() = text_view_contents(&style_for_edit_state);
        } else {
            original_class_for_selection.borrow_mut().clear();
            original_style_for_selection.borrow_mut().clear();
        }
        apply_class_for_edit_state.set_sensitive(false);
        apply_style_for_edit_state.set_sensitive(false);
    });

    let computed_details = gtk::ListBox::new();
    computed_details.set_selection_mode(gtk::SelectionMode::None);
    computed_details.add_css_class("boxed-list");
    let computed_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(180)
        .max_content_height(320)
        .child(&computed_details)
        .build();
    let computed_report = Rc::new(RefCell::new(String::new()));
    let computed_search = gtk::SearchEntry::new();
    computed_search.set_placeholder_text(Some("Filtrer les propriétés calculées…"));
    computed_search.set_hexpand(true);
    let report_for_search = computed_report.clone();
    let view_for_search = computed_details.clone();
    computed_search.connect_search_changed(move |entry| {
        populate_computed_style(
            &view_for_search,
            &report_for_search.borrow(),
            entry.text().as_str(),
        );
    });
    let computed_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    computed_actions.set_margin_start(0);
    let computed_refresh = gtk::Button::from_icon_name("view-refresh-symbolic");
    computed_refresh.set_tooltip_text(Some("Actualiser les styles calculés"));
    let computed_view = computed_details.clone();
    let computed_path = selected_path.clone();
    let computed_status = status.clone();
    let report_for_refresh = computed_report.clone();
    let search_for_refresh = computed_search.clone();
    let layout_values: [gtk::Label; 12] = std::array::from_fn(|_| gtk::Label::new(Some("—")));
    for label in &layout_values {
        label.add_css_class("title-3");
        label.add_css_class("monospace");
        label.set_halign(gtk::Align::Center);
    }
    let layout_size = gtk::Label::new(Some("— × —"));
    layout_size.add_css_class("title-3");
    layout_size.add_css_class("monospace");
    layout_size.set_halign(gtk::Align::Center);

    let layout_values_for_refresh = layout_values.clone();
    let layout_size_for_refresh = layout_size.clone();
    let layout_values_for_selection = layout_values.clone();
    let layout_size_for_selection = layout_size.clone();
    list.connect_row_selected(move |_, row| {
        if row.is_some() {
            for label in &layout_values_for_selection {
                label.set_text("—");
            }
            layout_size_for_selection.set_text("— × —");
        }
    });
    computed_refresh.connect_clicked(move |_| {
        let Some(path) = computed_path.borrow().clone() else {
            computed_status.set_text("Sélectionnez d’abord un acteur.");
            computed_status.add_css_class("error");
            return;
        };
        match refresh_computed_style(
            &path,
            &computed_view,
            &report_for_refresh,
            &search_for_refresh,
            &layout_values_for_refresh,
            &layout_size_for_refresh,
        ) {
            Ok(()) => {
                computed_status.set_text("Styles calculés actualisés.");
                computed_status.remove_css_class("error");
            }
            Err(error) => {
                computed_status.set_text(&format!("Échec de lecture du style calculé : {error}"));
                computed_status.add_css_class("error");
            }
        }
    });

    let path_for_live_style = selected_path.clone();
    let list_for_live_style = computed_details.clone();
    let report_for_live_style = computed_report.clone();
    let search_for_live_style = computed_search.clone();
    let values_for_live_style = layout_values.clone();
    let size_for_live_style = layout_size.clone();
    let status_for_live_style = status.clone();
    list.connect_row_selected(move |_, row| {
        let Some(path) = path_for_live_style
            .borrow()
            .clone()
            .filter(|_| row.is_some())
        else {
            *report_for_live_style.borrow_mut() = String::new();
            populate_computed_style(
                &list_for_live_style,
                "",
                search_for_live_style.text().as_str(),
            );
            for label in &values_for_live_style {
                label.set_text("—");
            }
            size_for_live_style.set_text("— × —");
            return;
        };

        match refresh_computed_style(
            &path,
            &list_for_live_style,
            &report_for_live_style,
            &search_for_live_style,
            &values_for_live_style,
            &size_for_live_style,
        ) {
            Ok(()) => {
                status_for_live_style
                    .set_text("Styles calculés synchronisés avec l’acteur sélectionné.");
                status_for_live_style.remove_css_class("error");
            }
            Err(error) => {
                status_for_live_style
                    .set_text(&format!("Échec de lecture du style calculé : {error}"));
                status_for_live_style.add_css_class("error");
            }
        }
    });
    let content_frame = gtk::Frame::new(Some("Contenu"));
    content_frame.set_child(Some(&layout_size));

    let padding_grid = gtk::Grid::new();
    padding_grid.set_row_spacing(8);
    padding_grid.set_column_spacing(16);
    padding_grid.set_halign(gtk::Align::Center);
    padding_grid.attach(&layout_values[8], 1, 0, 1, 1);
    padding_grid.attach(&layout_values[11], 0, 1, 1, 1);
    padding_grid.attach(&content_frame, 1, 1, 1, 1);
    padding_grid.attach(&layout_values[9], 2, 1, 1, 1);
    padding_grid.attach(&layout_values[10], 1, 2, 1, 1);
    let padding_frame = gtk::Frame::new(Some("Padding"));
    padding_frame.set_child(Some(&padding_grid));

    let border_grid = gtk::Grid::new();
    border_grid.set_row_spacing(8);
    border_grid.set_column_spacing(16);
    border_grid.set_halign(gtk::Align::Center);
    border_grid.attach(&layout_values[4], 1, 0, 1, 1);
    border_grid.attach(&layout_values[7], 0, 1, 1, 1);
    border_grid.attach(&padding_frame, 1, 1, 1, 1);
    border_grid.attach(&layout_values[5], 2, 1, 1, 1);
    border_grid.attach(&layout_values[6], 1, 2, 1, 1);
    let border_frame = gtk::Frame::new(Some("Bordure"));
    border_frame.set_child(Some(&border_grid));

    let margin_grid = gtk::Grid::new();
    margin_grid.set_row_spacing(8);
    margin_grid.set_column_spacing(16);
    margin_grid.set_halign(gtk::Align::Center);
    margin_grid.attach(&layout_values[0], 1, 0, 1, 1);
    margin_grid.attach(&layout_values[3], 0, 1, 1, 1);
    margin_grid.attach(&border_frame, 1, 1, 1, 1);
    margin_grid.attach(&layout_values[1], 2, 1, 1, 1);
    margin_grid.attach(&layout_values[2], 1, 2, 1, 1);
    let layout_box = gtk::Frame::new(Some("Modèle de boîte"));
    layout_box.set_child(Some(&margin_grid));
    layout_box.set_margin_top(24);
    layout_box.set_margin_bottom(24);
    layout_box.set_margin_start(24);
    layout_box.set_margin_end(24);
    let layout_note = gtk::Label::new(Some(
        "Valeurs par côté fournies par St.ThemeNode, en pixels physiques. Actualisez le style calculé pour les lire.",
    ));
    layout_note.set_wrap(true);
    layout_note.add_css_class("dim-label");
    layout_note.set_margin_start(24);
    layout_note.set_margin_end(24);
    let layout_panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    layout_panel.append(&layout_box);
    layout_panel.append(&layout_note);

    computed_actions.append(&computed_search);
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
    theme_actions.set_margin_start(0);
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

    let selection_header = gtk::Box::new(gtk::Orientation::Vertical, 4);
    selection_header.add_css_class("card");
    selection_header.set_margin_top(8);
    selection_header.set_margin_bottom(4);
    selection_header.set_margin_start(8);
    selection_header.set_margin_end(8);
    selection_header.append(&selected_actor_title);
    selection_header.append(&selected_actor_style);

    let selection_path_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    selection_path_row.append(&selected_actor_path);
    let copy_actor_path = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy_actor_path.set_tooltip_text(Some("Copier le chemin de l’acteur sélectionné"));
    copy_actor_path.set_sensitive(false);
    let path_for_copy = selected_path.clone();
    copy_actor_path.connect_clicked(move |_| {
        if let (Some(path), Some(display)) = (
            path_for_copy.borrow().as_ref(),
            gtk::gdk::Display::default(),
        ) {
            display.clipboard().set_text(path);
        }
    });
    let copy_for_selection = copy_actor_path.clone();
    let path_for_selection_state = selected_path.clone();
    list.connect_row_selected(move |_, row| {
        copy_for_selection
            .set_sensitive(row.is_some() && path_for_selection_state.borrow().is_some());
    });
    selection_path_row.append(&copy_actor_path);
    selection_header.append(&selection_path_row);

    let properties_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    properties_panel.set_margin_top(12);
    properties_panel.set_margin_bottom(12);
    properties_panel.set_margin_start(12);
    properties_panel.set_margin_end(12);
    let properties_title = gtk::Label::new(Some("Propriétés de l’acteur"));
    properties_title.add_css_class("heading");
    properties_title.set_halign(gtk::Align::Start);
    properties_panel.append(&properties_title);
    properties_panel.append(&details_scroll);

    let inspector_tabs = gtk::Notebook::new();
    inspector_tabs.set_scrollable(true);
    inspector_tabs.set_hexpand(true);
    inspector_tabs.set_vexpand(true);

    let inspector_keys = gtk::EventControllerKey::new();
    let tabs_for_keys = inspector_tabs.clone();
    let search_for_tabs = search.clone();
    let picker_for_keys = pick_actor.clone();
    inspector_keys.connect_key_pressed(move |_, key, _, modifiers| {
        match inspector_shortcut(key, modifiers) {
            Some(InspectorShortcut::ClearSearch) if !search_for_tabs.text().is_empty() => {
                search_for_tabs.set_text("");
                glib::Propagation::Stop
            }
            Some(InspectorShortcut::PickActor) => {
                picker_for_keys.emit_clicked();
                glib::Propagation::Stop
            }
            Some(InspectorShortcut::FocusSearch) => {
                search_for_tabs.grab_focus();
                glib::Propagation::Stop
            }
            Some(InspectorShortcut::SelectTab(index)) => {
                tabs_for_keys.set_current_page(Some(index));
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });
    inspector_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    page.add_controller(inspector_keys);
    inspector_tabs.set_tooltip_text(Some(
        "Raccourcis : Ctrl+1 Styles · Ctrl+2 Computed · Ctrl+3 Layout · Ctrl+F rechercher · Échap effacer la recherche · Ctrl+Maj+C sélectionner un acteur",
    ));

    let declarations_heading = gtk::Label::new(Some("Déclarations de element.style"));
    declarations_heading.add_css_class("heading");
    declarations_heading.set_halign(gtk::Align::Start);
    let class_section = inspector_section("Classes CSS", &class_row, true);
    let inline_style_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    inline_style_content.append(&declarations_heading);
    inline_style_content.append(&declaration_filter);
    inline_style_content.append(&declaration_scroll);
    inline_style_content.append(&style_row);
    let inline_style_section = inspector_section("element.style", &inline_style_content, true);

    let styles_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    styles_panel.set_margin_top(12);
    styles_panel.set_margin_bottom(12);
    styles_panel.set_margin_start(12);
    styles_panel.set_margin_end(12);
    let style_tools = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let copy_classes = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy_classes.set_tooltip_text(Some("Copier les classes CSS"));
    let classes_for_copy = class_input.clone();
    copy_classes.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            display
                .clipboard()
                .set_text(classes_for_copy.text().as_str());
        }
    });
    let copy_inline_style = gtk::Button::from_icon_name("edit-copy-symbolic");
    copy_inline_style.set_tooltip_text(Some("Copier le style inline"));
    let style_for_copy = style_input.clone();
    copy_inline_style.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            display
                .clipboard()
                .set_text(&text_view_contents(&style_for_copy));
        }
    });
    let class_copy_state = copy_classes.clone();
    let style_copy_state = copy_inline_style.clone();
    list.connect_row_selected(move |_, row| {
        let enabled = row.is_some();
        class_copy_state.set_sensitive(enabled);
        style_copy_state.set_sensitive(enabled);
    });
    copy_classes.set_sensitive(false);
    copy_inline_style.set_sensitive(false);
    style_tools.append(&copy_classes);
    style_tools.append(&copy_inline_style);

    let declarations_for_filter = declaration_list.clone();
    let selected_css_for_filter = selected_inline_css.clone();
    let style_for_filter = style_input.clone();
    let updating_for_filter = updating_declaration.clone();
    declaration_filter.connect_search_changed(move |entry| {
        populate_inline_declarations(
            &declarations_for_filter,
            &selected_css_for_filter.borrow(),
            entry.text().as_str(),
            &style_for_filter,
            &updating_for_filter,
        );
    });

    styles_panel.append(&style_tools);
    styles_panel.append(&class_section);
    styles_panel.append(&inline_style_section);
    inspector_tabs.append_page(&styles_panel, Some(&gtk::Label::new(Some("Styles"))));

    let computed_panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    computed_panel.set_margin_top(12);
    computed_panel.set_margin_bottom(12);
    computed_panel.set_margin_start(12);
    computed_panel.set_margin_end(12);
    computed_panel.append(&computed_actions);
    computed_panel.append(&computed_scroll);
    inspector_tabs.append_page(&computed_panel, Some(&gtk::Label::new(Some("Computed"))));
    inspector_tabs.append_page(&layout_panel, Some(&gtk::Label::new(Some("Layout"))));

    let theme_panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    theme_panel.set_margin_top(12);
    theme_panel.set_margin_bottom(12);
    theme_panel.set_margin_start(12);
    theme_panel.set_margin_end(12);
    theme_panel.append(&theme_actions);
    theme_panel.append(&custom_css_scroll);
    theme_panel.append(&theme_scroll);
    inspector_tabs.append_page(&theme_panel, Some(&gtk::Label::new(Some("Thème"))));
    inspector_tabs.append_page(
        &properties_panel,
        Some(&gtk::Label::new(Some("Propriétés"))),
    );

    let inspector_scroll = gtk::ScrolledWindow::builder()
        .min_content_width(340)
        .hexpand(true)
        .vexpand(true)
        .child(&inspector_tabs)
        .build();

    let inspector_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    inspector_panel.append(&selection_header);
    inspector_panel.append(&inspector_scroll);

    let tree_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    tree_panel.append(&list_scroll);
    let breadcrumb_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    breadcrumb_bar.add_css_class("toolbar");
    breadcrumb_bar.append(&tree_breadcrumb);
    tree_panel.append(&breadcrumb_bar);

    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_start_child(Some(&tree_panel));
    split.set_end_child(Some(&inspector_panel));
    split.set_resize_start_child(false);
    split.set_shrink_start_child(false);
    split.set_position(330);
    split.set_vexpand(true);

    page.append(&actions);
    page.append(&split);
    page.append(&status);
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

    let console_toggle = gtk::ToggleButton::new();
    console_toggle.set_child(Some(&gtk::Image::from_icon_name(
        "utilities-terminal-symbolic",
    )));
    console_toggle.set_tooltip_text(Some(
        "Afficher la console GJS intégrée (exécute du code dans GNOME Shell)",
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
        actor_breadcrumb, actor_detail_summary, actor_first_child_index, actor_index_by_path,
        actor_inspector_title, actor_matches, actor_parent_path, actor_path,
        actor_property_has_changed, actor_selection_index, actor_style_summary,
        inspector_shortcut, InspectorShortcut, computed_style_entries, editable_actor_properties, extension_is_discovered, flatten_actors,
        format_actor_details, format_actor_details_filtered, format_computed_style,
        format_console_entry, layout_metric, remember_console_command, string_reply, text_differs,
        update_inline_declaration,
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
    fn actor_tree_keyboard_navigation_resolves_parent_and_first_child() {
        let nodes = vec![
            serde_json::json!({"path": ""}),
            serde_json::json!({"path": "0"}),
            serde_json::json!({"path": "0/1"}),
            serde_json::json!({"path": "0/1/2"}),
            serde_json::json!({"path": "0/3"}),
        ];

        assert_eq!(actor_parent_path(""), None);
        assert_eq!(actor_parent_path("0"), Some(""));
        assert_eq!(actor_parent_path("0/1/2"), Some("0/1"));
        assert_eq!(actor_first_child_index(&nodes, ""), Some(1));
        assert_eq!(actor_first_child_index(&nodes, "0/1"), Some(3));
        assert_eq!(actor_first_child_index(&nodes, "0/3"), None);
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
    fn property_editor_is_enabled_only_when_a_value_changes() {
        let node = serde_json::json!({
            "properties": [
                {"name": "label", "type": "gchararray", "writable": true, "value": "OK"}
            ]
        });

        assert!(!actor_property_has_changed(&node, "label", "OK"));
        assert!(actor_property_has_changed(&node, "label", "Cancel"));
        assert!(!actor_property_has_changed(&node, "missing", "value"));
    }

    #[test]
    fn inline_style_declarations_can_be_edited_and_toggled() {
        let css = "color: red; padding: 4px;";

        assert_eq!(
            update_inline_declaration(css, "color", "blue", true),
            "padding: 4px; color: blue;"
        );
        assert_eq!(
            update_inline_declaration(css, "color", "red", false),
            "padding: 4px;"
        );
        assert_eq!(
            update_inline_declaration("padding: 4px;", "color", "blue", true),
            "padding: 4px; color: blue;"
        );
    }

    #[test]
    fn css_edit_actions_require_a_changed_value() {
        assert!(!text_differs("button primary", "button primary"));
        assert!(text_differs("button", "button primary"));
        assert!(text_differs("color: red;", ""));
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
    fn actor_breadcrumb_shows_the_selected_actor_ancestry() {
        let nodes = vec![
            serde_json::json!({"name": "stage", "path": ""}),
            serde_json::json!({"name": "panel", "path": "0"}),
            serde_json::json!({"name": "button", "path": "0/2"}),
        ];

        assert_eq!(
            actor_breadcrumb(&nodes, "0/2"),
            "stage  ›  panel  ›  button"
        );
        assert_eq!(actor_breadcrumb(&nodes, ""), "stage");
        assert_eq!(actor_breadcrumb(&nodes, "9"), "stage");
    }

    #[test]
    fn actor_inspector_title_shows_name_and_type() {
        let node = serde_json::json!({"name": "Panel", "type": "StBoxLayout"});
        assert_eq!(actor_inspector_title(&node), "Panel  ·  StBoxLayout");
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
    fn formats_layout_metrics_without_inventing_unavailable_values() {
        let value = serde_json::json!({
            "width": 120,
            "height": 48.5,
            "marginTop": 1,
            "borderRight": 2,
            "paddingLeft": 3,
            "paddingBottom": null
        });

        assert_eq!(layout_metric(&value, "width"), "120 px");
        assert_eq!(layout_metric(&value, "height"), "48.5 px");
        assert_eq!(layout_metric(&value, "marginTop"), "1 px");
        assert_eq!(layout_metric(&value, "borderRight"), "2 px");
        assert_eq!(layout_metric(&value, "paddingLeft"), "3 px");
        assert_eq!(layout_metric(&value, "paddingBottom"), "—");
        assert_eq!(layout_metric(&value, "unknown"), "—");
    }

    #[test]
    fn maps_inspector_keyboard_shortcuts_without_intercepting_other_keys() {
        use gtk::gdk::{Key, ModifierType};

        let control = ModifierType::CONTROL_MASK;
        let control_shift = control | ModifierType::SHIFT_MASK;

        assert_eq!(
            inspector_shortcut(Key::_1, control),
            Some(InspectorShortcut::SelectTab(0))
        );
        assert_eq!(
            inspector_shortcut(Key::_2, control),
            Some(InspectorShortcut::SelectTab(1))
        );
        assert_eq!(
            inspector_shortcut(Key::_3, control),
            Some(InspectorShortcut::SelectTab(2))
        );
        assert_eq!(
            inspector_shortcut(Key::f, control),
            Some(InspectorShortcut::FocusSearch)
        );
        assert_eq!(
            inspector_shortcut(Key::c, control_shift),
            Some(InspectorShortcut::PickActor)
        );
        assert_eq!(
            inspector_shortcut(Key::Escape, ModifierType::empty()),
            Some(InspectorShortcut::ClearSearch)
        );
        assert_eq!(inspector_shortcut(Key::x, control), None);
        assert_eq!(inspector_shortcut(Key::f, ModifierType::empty()), None);
    }

    #[test]
    fn formats_actor_style_classes_and_pseudo_classes_for_the_inspector() {
        let node = serde_json::json!({
            "styleClass": "button primary",
            "pseudoClass": "hover focus"
        });
        assert_eq!(actor_style_summary(&node), ".button .primary :hover :focus");
        assert_eq!(
            actor_style_summary(&serde_json::json!({})),
            "Aucune classe ni pseudo-classe"
        );
    }

    #[test]
    fn formats_computed_style_properties_for_the_inspector() {
        let report = format_computed_style(r#"{"font":"Sans 12px","width":640}"#).unwrap();
        assert!(report.contains("font: \"Sans 12px\""));
        assert!(report.contains("width: 640"));
        assert_eq!(
            computed_style_entries(&report, "WIDTH"),
            vec![(String::from("width"), String::from("640"))]
        );
        assert_eq!(
            computed_style_entries(&report, "sans"),
            vec![(String::from("font"), String::from("\"Sans 12px\""))]
        );
        assert_eq!(computed_style_entries(&report, "").len(), 2);
        assert!(computed_style_entries(&report, "missing").is_empty());
    }

    #[test]
    fn computed_style_entries_are_separate_filterable_properties() {
        let report = "font: Sans 12px\nwidth: 640\ncolor: rgba(1, 2, 3, 1)";
        assert_eq!(
            computed_style_entries(report, "WIDTH"),
            vec![(String::from("width"), String::from("640"))]
        );
        assert_eq!(
            computed_style_entries(report, "rgba"),
            vec![(String::from("color"), String::from("rgba(1, 2, 3, 1)"))]
        );
        assert_eq!(computed_style_entries(report, "").len(), 3);
        assert!(computed_style_entries(report, "missing").is_empty());
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
