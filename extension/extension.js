import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import St from 'gi://St';

export default class GnomeInspectorExtension extends Extension {
    enable() {
        this._indicator = new PanelMenu.Button(0.0, this.metadata.name);
        this._indicator.add_child(new St.Label({
            text: 'Inspector',
            y_align: 2,
        }));
        this._indicator.menu.addMenuItem(
            new PopupMenu.PopupMenuItem('GNOME Inspector agent enabled')
        );

        Main.panel.addToStatusArea(this.metadata.uuid, this._indicator);
    }

    disable() {
        this._indicator?.destroy();
        this._indicator = null;
    }
}
