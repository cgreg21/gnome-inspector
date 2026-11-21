import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import { buildActorTree } from './actor-tree.js';
import { buildStyleTree } from './style-tree.js';

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
    <method name="GetStyleTree">
      <arg type="s" direction="out"/>
    </method>
    <method name="GetDiagnostics">
      <arg type="s" direction="out"/>
    </method>
    <method name="EvaluateGjs">
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
  </interface>
</node>`;

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
        return JSON.stringify(buildActorTree(global.stage));
    }

    GetStyleTree() {
        return JSON.stringify(buildStyleTree(global.stage));
    }

    GetDiagnostics() {
        return JSON.stringify({
            shellVersion: Config.PACKAGE_VERSION,
            sessionType: GLib.getenv('XDG_SESSION_TYPE') || 'unknown',
            waylandDisplay: GLib.getenv('WAYLAND_DISPLAY') || null,
            display: GLib.getenv('DISPLAY') || null,
            stage: {
                width: global.stage.width,
                height: global.stage.height,
            },
            monitorCount: Main.layoutManager.monitors.length,
            extension: {
                uuid: this.metadata.uuid,
                version: this.metadata.version,
                path: this.path,
            },
        });
    }

    EvaluateGjs(source) {
        try {
            const result = eval(source);
            if (result === undefined)
                return 'undefined';
            if (typeof result === 'string')
                return result;
            return JSON.stringify(result, null, 2) ?? String(result);
        } catch (error) {
            return error?.stack || String(error);
        }
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
