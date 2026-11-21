use adw::prelude::*;

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

fn refresh_actor_tree(view: &gtk::TextView, status: &gtk::Label) {
    match actor_tree() {
        Ok(json) => {
            let content = serde_json::from_str::<serde_json::Value>(&json)
                .ok()
                .and_then(|value| serde_json::to_string_pretty(&value).ok())
                .unwrap_or(json);
            view.buffer().set_text(&content);
            status.set_text("Arbre des acteurs actualisé.");
            status.remove_css_class("error");
        }
        Err(error) => {
            status.set_text(&format!("Échec de l’inspection : {error}"));
            status.add_css_class("error");
        }
    }
}

fn actors_page() -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
    page.append(&page_heading(
        "Acteurs",
        "Explorez la hiérarchie Clutter/St de la scène GNOME Shell.",
    ));

    let status = gtk::Label::new(Some("Aucune capture effectuée."));
    status.set_halign(gtk::Align::Start);
    status.set_margin_start(24);
    status.add_css_class("dim-label");

    let view = gtk::TextView::new();
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(gtk::WrapMode::None);
    view.set_left_margin(12);
    view.set_top_margin(12);

    let scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&view)
        .build();

    let refresh = gtk::Button::with_label("Actualiser l’arbre");
    refresh.add_css_class("suggested-action");
    refresh.set_halign(gtk::Align::Start);
    let view_for_refresh = view.clone();
    let status_for_refresh = status.clone();
    refresh.connect_clicked(move |_| {
        refresh_actor_tree(&view_for_refresh, &status_for_refresh);
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
    stack.add_titled(
        &actors_page(),
        Some("actors"),
        "Acteurs",
    );
    stack.add_titled(
        &info_page(
            "CSS et thèmes",
            "Inspectez les styles GNOME Shell, les classes CSS et les thèmes actifs.",
            "Le module CSS sera connecté à l’agent Shell dans la prochaine étape de développement.",
        ),
        Some("styles"),
        "CSS et thèmes",
    );
    stack.add_titled(
        &info_page(
            "Console GJS",
            "Exécutez et suivez du code JavaScript dans le contexte GNOME Shell.",
            "La console interactive et son canal d’exécution sécurisé restent à implémenter.",
        ),
        Some("console"),
        "Console GJS",
    );
    stack.add_titled(
        &info_page(
            "Diagnostics",
            "Consultez les erreurs, avertissements, journaux et informations de session.",
            "La collecte des journaux et les diagnostics détaillés seront ajoutés au module.",
        ),
        Some("diagnostics"),
        "Diagnostics",
    );
    stack.add_titled(
        &info_page(
            "Performances",
            "Suivez les ressources, les délais de rendu et l’activité de GNOME Shell.",
            "Les mesures de performances et leur historique restent à implémenter.",
        ),
        Some("performance"),
        "Performances",
    );

    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(Some(&stack));
    sidebar.set_size_request(210, -1);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    content.append(&header);

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
