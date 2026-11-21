use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

const BUS_NAME: &str = "io.github.cgreg21.GnomeInspector";
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

fn shell_version() -> Result<String, String> {
    string_reply(&shell_call("Ping")?, "Invalid Shell version reply.")
}

fn actor_tree() -> Result<String, String> {
    string_reply(&shell_call("GetActorTree")?, "Invalid actor tree reply.")
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

fn info_page(title: &str, description: &str, status: &str) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(title, description));

    let message = gtk::Label::new(Some(status));
    message.set_wrap(true);
    message.set_xalign(0.0);
    message.set_margin_start(24);
    message.set_margin_end(24);
    message.add_css_class("dim-label");
    page.append(&message);
    page
}

fn refresh_connection(status: &gtk::Label) {
    match shell_version() {
        Ok(version) => {
            status.set_text(&format!("Connecté à GNOME Shell {version}"));
            status.remove_css_class("error");
            status.add_css_class("success");
        }
        Err(error) => {
            status.set_text(&format!("Déconnecté — {error}"));
            status.remove_css_class("success");
            status.add_css_class("error");
        }
    }
}

fn flatten_actors(node: &serde_json::Value, depth: usize, rows: &mut Vec<(serde_json::Value, usize)>) {
    rows.push((node.clone(), depth));
    if let Some(children) = node.get("children").and_then(serde_json::Value::as_array) {
        for child in children {
            flatten_actors(child, depth + 1, rows);
        }
    }
}

fn show_actor_details(view: &gtk::TextView, node: &serde_json::Value) {
    let details = serde_json::to_string_pretty(node).unwrap_or_else(|_| node.to_string());
    view.buffer().set_text(&details);
}

fn refresh_actor_tree(
    list: &gtk::ListBox,
    details: &gtk::TextView,
    selected_nodes: &Rc<RefCell<Vec<serde_json::Value>>>,
    status: &gtk::Label,
) {
    match actor_tree() {
        Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
            Ok(snapshot) => {
                let mut rows = Vec::new();
                if let Some(root) = snapshot.get("root") {
                    flatten_actors(root, 0, &mut rows);
                }

                while let Some(child) = list.first_child() {
                    list.remove(&child);
                }

                let nodes = rows.iter().map(|(node, _)| node.clone()).collect::<Vec<_>>();
                *selected_nodes.borrow_mut() = nodes;

                for (node, depth) in rows {
                    let name = node.get("name").and_then(serde_json::Value::as_str).unwrap_or("Acteur");
                    let kind = node.get("type").and_then(serde_json::Value::as_str).unwrap_or("Unknown");
                    let label = gtk::Label::new(Some(&format!("{}{}  ·  {}", "  ".repeat(depth), name, kind)));
                    label.set_halign(gtk::Align::Start);
                    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    label.set_margin_top(5);
                    label.set_margin_bottom(5);
                    label.set_margin_start(8);
                    label.set_margin_end(8);
                    list.append(&label);
                }

                let actor_count = snapshot.get("actorCount").and_then(serde_json::Value::as_u64).unwrap_or(rows_len(&selected_nodes));
                let incomplete = snapshot.get("incomplete").and_then(serde_json::Value::as_bool).unwrap_or(false);
                let truncated = snapshot.get("truncated").and_then(serde_json::Value::as_bool).unwrap_or(false);
                status.set_text(&format!(
                    "{actor_count} acteurs · {}{}",
                    if incomplete { "capture incomplète" } else { "capture complète" },
                    if truncated { " · limite de parcours atteinte" } else { "" }
                ));

                if let Some(root) = selected_nodes.borrow().first() {
                    show_actor_details(details, root);
                } else {
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
    let details_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&details)
        .build();

    let selected_nodes = Rc::new(RefCell::new(Vec::<serde_json::Value>::new()));
    let nodes_for_selection = selected_nodes.clone();
    let details_for_selection = details.clone();
    list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let index = row.index() as usize;
            if let Some(node) = nodes_for_selection.borrow().get(index) {
                show_actor_details(&details_for_selection, node);
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
    refresh.connect_clicked(move |_| {
        refresh_actor_tree(
            &list_for_refresh,
            &details_for_refresh,
            &nodes_for_refresh,
            &status_for_refresh,
        );
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_margin_start(24);
    actions.set_margin_end(24);
    actions.append(&refresh);

    let split = gtk::Paned::new(gtk::Orientation::Horizontal);
    split.set_start_child(Some(&list_scroll));
    split.set_end_child(Some(&details_scroll));
    split.set_resize_start_child(false);
    split.set_shrink_start_child(false);
    split.set_position(340);

    page.append(&actions);
    page.append(&status);
    page.append(&split);
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

    let retry = gtk::Button::with_label("Tester la connexion");
    retry.set_halign(gtk::Align::Start);
    let connection_for_retry = connection.clone();
    retry.connect_clicked(move |_| refresh_connection(&connection_for_retry));
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
        .default_width(1100)
        .default_height(720)
        .build();

    let connection = gtk::Label::new(Some("Vérification de la connexion…"));
    connection.set_halign(gtk::Align::Start);
    refresh_connection(&connection);

    let stack = gtk::Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);

    stack.add_titled(&overview_page(&connection), Some("overview"), "Vue d’ensemble");
    stack.add_titled(&actors_page(), Some("actors"), "Acteurs");
    stack.add_titled(
        &info_page(
            "CSS et thèmes",
            "Inspectez les styles GNOME Shell, les classes CSS et les thèmes actifs.",
            "Le moteur d’inspection CSS sera connecté à l’agent Shell.",
        ),
        Some("styles"),
        "CSS et thèmes",
    );
    stack.add_titled(
        &info_page(
            "Console GJS",
            "Exécutez et suivez du code JavaScript dans le contexte GNOME Shell.",
            "La console interactive et son canal d’exécution restent à construire.",
        ),
        Some("console"),
        "Console GJS",
    );
    stack.add_titled(
        &info_page(
            "Diagnostics",
            "Consultez les erreurs, avertissements, journaux et informations de session.",
            "La collecte des journaux et les diagnostics détaillés seront ajoutés ici.",
        ),
        Some("diagnostics"),
        "Diagnostics",
    );
    stack.add_titled(
        &info_page(
            "Performances",
            "Suivez les ressources, les délais de rendu et l’activité de GNOME Shell.",
            "Les mesures de performances et leur historique seront ajoutés ici.",
        ),
        Some("performance"),
        "Performances",
    );

    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(Some(&stack));
    sidebar.set_size_request(210, -1);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&adw::HeaderBar::new());

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.append(&sidebar);
    body.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    body.append(&stack);
    content.append(&body);

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
    use super::string_reply;
    use glib::variant::ToVariant;

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
}
