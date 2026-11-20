import Gio from 'gi://Gio';
import St from 'gi://St';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';

const BUS_NAME = 'io.github.cgreg21.GnomeInspector';
const OBJECT_PATH = '/io/github/cgreg21/GnomeInspector';
const INTERFACE_XML = `
<node>
  <interface name="io.github.cgreg21.GnomeInspector">
    <method name="Ping">
      <arg type="s" direction="out"/>
    </method>
    <method name="GetActorTree">
      <arg type="s" direction="out"/>
    </method>
  </interface>
</node>`;
const MAX_ACTORS = 1000;
const MAX_DEPTH = 12;

function actorInfo(actor, depth, budget) {
    const info = {
        name: actor.get_name() || actor.constructor.name || 'Actor',
        type: actor.constructor.name || 'Actor',
        visible: actor.visible,
        reactive: actor.reactive,
        x: Math.round(actor.x),
        y: Math.round(actor.y),
        width: Math.round(actor.width),
        height: Math.round(actor.height),
        children: [],
    };

    if (depth >= MAX_DEPTH || budget.count >= MAX_ACTORS)
        return info;

    for (const child of actor.get_children()) {
        if (budget.count >= MAX_ACTORS)
            break;

        budget.count++;
        info.children.push(actorInfo(child, depth + 1, budget));
    }

    return info;
}

export default class GnomeInspectorExtension extends Extension {
    enable() {
        this._indicator = new PanelMenu.Button(0.0, this.metadata.name);
        this._indicator.add_child(new St.Label({ text: 'Inspector' }));
        this._indicator.menu.addMenuItem(
            new PopupMenu.PopupMenuItem('GNOME Inspector agent enabled')
        );
        Main.panel.addToStatusArea(this.metadata.uuid, this._indicator);

        this._nameOwnerId = Gio.bus_own_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            connection => {
                this._dbus = Gio.DBusExportedObject.wrapJSObject(
                    INTERFACE_XML,
                    this
                );
                this._dbus.export(connection, OBJECT_PATH);
            },
            null,
            () => {
                this._dbus?.unexport();
                this._dbus = null;
            }
        );
    }

    Ping() {
        return Config.PACKAGE_VERSION;
    }

    GetActorTree() {
        const budget = { count: 1 };
        return JSON.stringify(actorInfo(global.stage, 0, budget));
    }

    disable() {
        if (this._nameOwnerId) {
            Gio.bus_unown_name(this._nameOwnerId);
            this._nameOwnerId = null;
        }
        this._dbus?.unexport();
        this._dbus = null;
        this._indicator?.destroy();
        this._indicator = null;
    }
}
