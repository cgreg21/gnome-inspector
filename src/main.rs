use adw::prelude::*;
use gtk::prelude::*;

const BUS_NAME: &str = "io.github.cgreg21.GnomeInspector";
const OBJECT_PATH: &str = "/io/github/cgreg21/GnomeInspector";
const INTERFACE: &str = "io.github.cgreg21.GnomeInspector";

fn shell_version() -> Result<String, String> {
    let connection =
        gio::bus_get_sync(gio::BusType::Session, None).map_err(|error| error.to_string())?;

    let reply = connection
        .call_sync(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            "Ping",
            None,
            None,
            gio::DBusCallFlags::NONE,
            1_500,
            None,
        )
        .map_err(|error| error.to_string())?;

    reply
        .get::<(String,)>()
        .map(|(version,)| version)
        .ok_or_else(|| "The Shell agent returned an invalid response.".to_string())
}

fn refresh_status(status: &gtk::Label) {
    match shell_version() {
        Ok(version) => status.set_text(&format!("Connected to GNOME Shell {version}.")),
        Err(error) => status.set_text(&format!(
            "GNOME Shell is not connected. Enable the GNOME Inspector extension.\n\n{error}"
        )),
    }
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
    status.set_margin_top(48);
    status.set_margin_bottom(24);
    status.set_margin_start(24);
    status.set_margin_end(24);
    refresh_status(&status);

    let retry = gtk::Button::with_label("Retry connection");
    retry.set_halign(gtk::Align::Center);
    let status_for_retry = status.clone();
    retry.connect_clicked(move |_| refresh_status(&status_for_retry));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&header);
    content.append(&status);
    content.append(&retry);
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
