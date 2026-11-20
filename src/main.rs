use adw::prelude::*;
use gtk::prelude::*;

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("GNOME Inspector")
        .default_width(900)
        .default_height(640)
        .build();

    let header = adw::HeaderBar::new();
    let status = adw::StatusPage::builder()
        .icon_name("applications-engineering-symbolic")
        .title("GNOME Shell not connected")
        .description(
            "The desktop agent is not connected yet. Install and enable the GNOME Shell extension when the D-Bus transport is available.",
        )
        .build();
    status.set_vexpand(true);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&header);
    content.append(&status);
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
