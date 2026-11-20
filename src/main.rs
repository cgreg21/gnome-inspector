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

fn shell_version() -> Result<String, String> {
    shell_call("Ping")?
        .get::<(String,)>()
        .map(|(version,)| version)
        .ok_or_else(|| "The Shell agent returned an invalid version.".to_string())
}

fn actor_tree() -> Result<String, String> {
    shell_call("GetActorTree")?
        .get::<(String,)>()
        .map(|(tree,)| tree)
        .ok_or_else(|| "The Shell agent returned an invalid actor tree.".to_string())
}

fn refresh_status(status: &gtk::Label) {
    match shell_version() {
        Ok(version) => status.set_text(&format!("Connected to GNOME Shell {version}.")),
        Err(error) => status.set_text(&format!(
            "GNOME Shell is not connected. Enable the GNOME Inspector extension.\n\n{error}"
        )),
    }
}

fn refresh_actor_tree(view: &gtk::TextView) {
    let content = actor_tree().unwrap_or_else(|error| format!("Unable to inspect actors: {error}"));
    view.buffer().set_text(&content);
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("GNOME Inspector")
        .default_width(900)
        .default_height(640)
        .build();

    let header = adw::HeaderBar::new();
    let status = gtk::Label::new(None);
    status.set_wrap(true);
    status.set_margin_top(24);
    status.set_margin_bottom(12);
    status.set_margin_start(24);
    status.set_margin_end(24);
    refresh_status(&status);

    let retry = gtk::Button::with_label("Retry connection");
    retry.set_halign(gtk::Align::Center);
    let status_for_retry = status.clone();
    retry.connect_clicked(move |_| refresh_status(&status_for_retry));

    let tree_label = gtk::Label::new(Some("Shell actor tree (JSON)"));
    tree_label.set_halign(gtk::Align::Start);
    tree_label.set_margin_start(24);

    let tree = gtk::TextView::new();
    tree.set_editable(false);
    tree.set_cursor_visible(false);
    tree.set_monospace(true);
    tree.set_wrap_mode(gtk::WrapMode::None);

    let tree_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&tree)
        .build();

    let refresh = gtk::Button::with_label("Refresh actor tree");
    refresh.set_halign(gtk::Align::Center);
    let tree_for_refresh = tree.clone();
    refresh.connect_clicked(move |_| refresh_actor_tree(&tree_for_refresh));

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::Center);
    actions.append(&retry);
    actions.append(&refresh);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&header);
    content.append(&status);
    content.append(&actions);
    content.append(&tree_label);
    content.append(&tree_scroll);
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
