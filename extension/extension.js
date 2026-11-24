import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Config from 'resource:///org/gnome/shell/misc/config.js';
import { buildActorTree } from './actor-tree.js';
import { findActorPath } from './actor-picker.js';
import { buildComputedStyle, buildStyleTree, resolveActorPath } from './style-tree.js';
import { setActorProperty } from './property-editor.js';
import { parseProcessMemory } from './process-metrics.js';
import {
    readCustomStylesheet,
    reloadCustomStylesheets,
    saveCustomStylesheet,
} from './theme-stylesheets.js';

const BUS_NAME = 'io.github.cgreg21.GnomeInspector.Shell';
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
    <method name="StartActorPicker">
      <arg type="s" direction="out"/>
    </method>
    <method name="GetActorPickerResult">
      <arg type="s" direction="out"/>
    </method>
    <method name="CancelActorPicker">
      <arg type="s" direction="out"/>
    </method>
    <method name="GetStyleTree">
      <arg type="s" direction="out"/>
    </method>
    <method name="GetActorComputedStyle">
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="GetThemeInfo">
      <arg type="s" direction="out"/>
    </method>
    <method name="ReloadCustomStylesheets">
      <arg type="s" direction="out"/>
    </method>
    <method name="GetCustomStylesheet">
      <arg type="s" direction="out"/>
    </method>
    <method name="SaveCustomStylesheet">
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="SetActorProperty">
      <arg type="s" direction="in"/>
      <arg type="s" direction="in"/>
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="SetActorStyleClass">
      <arg type="s" direction="in"/>
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="SetActorInlineStyle">
      <arg type="s" direction="in"/>
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="GetDiagnostics">
      <arg type="s" direction="out"/>
    </method>
    <method name="EvaluateGjs">
      <arg type="s" direction="in"/>
      <arg type="s" direction="out"/>
    </method>
    <method name="GetPerformance">
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

        this._startPerformanceMonitoring();
    }

    Ping() {
        return Config.PACKAGE_VERSION;
    }

    GetActorTree() {
        return JSON.stringify(buildActorTree(global.stage));
    }

    StartActorPicker() {
        this._finishActorPicker({ status: 'cancelled' });
        this._actorPickerResult = { status: 'pending' };
        this._actorPickerSignalId = global.stage.connect('captured-event', (stage, event) => {
            const type = event.type();
            if (type === Clutter.EventType.KEY_PRESS
                && event.get_key_symbol() === Clutter.KEY_Escape) {
                this._finishActorPicker({ status: 'cancelled' });
                return Clutter.EVENT_STOP;
            }
            if (type !== Clutter.EventType.BUTTON_PRESS)
                return Clutter.EVENT_PROPAGATE;

            const [x, y] = event.get_coords();
            const actor = stage.get_actor_at_pos(Clutter.PickMode.ALL, x, y);
            const path = findActorPath(global.stage, actor);
            if (path === null)
                return Clutter.EVENT_PROPAGATE;

            this._highlightActor(actor);
            this._finishActorPicker({ status: 'selected', path });
            return Clutter.EVENT_STOP;
        });
        this._actorPickerTimeout = GLib.timeout_add_seconds(
            GLib.PRIORITY_DEFAULT,
            15,
            () => {
                this._actorPickerTimeout = null;
                this._finishActorPicker({ status: 'timeout' });
                return GLib.SOURCE_REMOVE;
            }
        );
        return JSON.stringify({ ok: true });
    }

    GetActorPickerResult() {
        return JSON.stringify(this._actorPickerResult || { status: 'idle' });
    }

    CancelActorPicker() {
        this._finishActorPicker({ status: 'cancelled' });
        return JSON.stringify({ ok: true });
    }

    _finishActorPicker(result) {
        if (this._actorPickerSignalId) {
            global.stage.disconnect(this._actorPickerSignalId);
            this._actorPickerSignalId = null;
        }
        if (this._actorPickerTimeout) {
            GLib.Source.remove(this._actorPickerTimeout);
            this._actorPickerTimeout = null;
        }
        this._actorPickerResult = result;
    }

    _highlightActor(actor) {
        this._clearActorHighlight();
        const [x, y] = actor.get_transformed_position();
        const [width, height] = actor.get_transformed_size();
        this._actorHighlight = new St.Widget({
            reactive: false,
            can_focus: false,
            style: 'border: 2px solid #3584e4; background-color: rgba(53, 132, 228, 0.12);',
        });
        this._actorHighlight.set_position(x, y);
        this._actorHighlight.set_size(width, height);
        Main.uiGroup.add_child(this._actorHighlight);
        this._actorHighlightTimeout = GLib.timeout_add_seconds(
            GLib.PRIORITY_DEFAULT,
            2,
            () => {
                this._actorHighlightTimeout = null;
                this._actorHighlight?.destroy();
                this._actorHighlight = null;
                return GLib.SOURCE_REMOVE;
            }
        );
    }

    _clearActorHighlight() {
        if (this._actorHighlightTimeout) {
            GLib.Source.remove(this._actorHighlightTimeout);
            this._actorHighlightTimeout = null;
        }
        this._actorHighlight?.destroy();
        this._actorHighlight = null;
    }

    GetStyleTree() {
        return JSON.stringify(buildStyleTree(global.stage));
    }

    GetActorComputedStyle(path) {
        const actor = resolveActorPath(global.stage, path);
        if (!actor || typeof actor.get_theme_node !== 'function')
            return JSON.stringify({ available: false, error: 'Actor path is invalid or the actor has no computed theme style.' });

        return JSON.stringify(buildComputedStyle(actor, St.Side));
    }

    GetThemeInfo() {
        const theme = St.ThemeContext.get_for_stage(global.stage)?.get_theme();
        if (!theme)
            return JSON.stringify({ available: false });

        const pathOf = file => {
            try {
                return file?.get_path?.() || file?.get_uri?.() || null;
            } catch {
                return null;
            }
        };

        return JSON.stringify({
            available: true,
            applicationStylesheet: pathOf(theme.get_application_stylesheet()),
            themeStylesheet: pathOf(theme.get_theme_stylesheet()),
            defaultStylesheet: pathOf(theme.get_default_stylesheet()),
            customStylesheets: theme.get_custom_stylesheets(),
        });
    }

    ReloadCustomStylesheets() {
        const theme = St.ThemeContext.get_for_stage(global.stage)?.get_theme();
        if (!theme)
            return JSON.stringify({ ok: false, reloaded: 0, errors: ['Theme unavailable.'] });

        return JSON.stringify(reloadCustomStylesheets(theme));
    }

    GetCustomStylesheet() {
        try {
            return readCustomStylesheet(Gio, GLib);
        } catch (error) {
            return '';
        }
    }

    SaveCustomStylesheet(css) {
        const theme = St.ThemeContext.get_for_stage(global.stage)?.get_theme();
        if (!theme)
            return JSON.stringify({ ok: false, error: 'Theme unavailable.' });

        try {
            return JSON.stringify(saveCustomStylesheet(theme, Gio, GLib, css));
        } catch (error) {
            return JSON.stringify({ ok: false, error: error?.message || String(error) });
        }
    }

    SetActorProperty(path, name, value) {
        const actor = resolveActorPath(global.stage, path);
        if (!actor)
            return JSON.stringify({ ok: false, error: 'Actor path is invalid.' });
        return JSON.stringify(setActorProperty(actor, name, value));
    }

    SetActorStyleClass(path, styleClass) {
        if (typeof styleClass !== 'string' || styleClass.length > 512)
            return JSON.stringify({ ok: false, error: 'CSS class list must be at most 512 characters.' });

        const actor = resolveActorPath(global.stage, path);
        if (!actor || typeof actor.set_style_class_name !== 'function')
            return JSON.stringify({ ok: false, error: 'Actor path is invalid or the actor does not support CSS classes.' });

        try {
            actor.set_style_class_name(styleClass);
            return JSON.stringify({ ok: true, path, styleClass });
        } catch (error) {
            return JSON.stringify({ ok: false, error: error?.message || String(error) });
        }
    }

    SetActorInlineStyle(path, css) {
        if (typeof css !== 'string' || css.length > 4096)
            return JSON.stringify({ ok: false, error: 'CSS must be a string of at most 4096 characters.' });

        const actor = resolveActorPath(global.stage, path);
        if (!actor || typeof actor.set_style !== 'function')
            return JSON.stringify({ ok: false, error: 'Actor path is invalid or the actor does not support inline styles.' });

        try {
            actor.set_style(css);
            return JSON.stringify({ ok: true, path });
        } catch (error) {
            return JSON.stringify({ ok: false, error: error?.message || String(error) });
        }
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

    _startPerformanceMonitoring() {
        this._performance = {
            renderedFrames: 0,
            lastRenderedFrames: 0,
            lastSampleAt: GLib.get_monotonic_time(),
            memory: this._readProcessMemory(),
            history: [],
        };
        this._performanceSignalId = global.stage.connect('after-paint', () => {
            this._performance.renderedFrames++;
        });
        this._performanceTimer = GLib.timeout_add_seconds(
            GLib.PRIORITY_DEFAULT,
            1,
            () => {
                const now = GLib.get_monotonic_time();
                const elapsedUs = now - this._performance.lastSampleAt;
                const frames = this._performance.renderedFrames
                    - this._performance.lastRenderedFrames;
                const memory = this._readProcessMemory();
                if (elapsedUs > 0) {
                    this._performance.history.push({
                        sampledAt: GLib.get_real_time(),
                        intervalUs: elapsedUs,
                        renderedFrames: frames,
                        framesPerSecond: Math.round(frames * 100000000 / elapsedUs) / 100,
                        residentMemoryBytes: memory?.residentBytes ?? null,
                        peakResidentMemoryBytes: memory?.peakResidentBytes ?? null,
                    });
                    if (this._performance.history.length > 60)
                        this._performance.history.shift();
                }
                this._performance.lastSampleAt = now;
                this._performance.lastRenderedFrames = this._performance.renderedFrames;
                this._performance.memory = memory;
                return GLib.SOURCE_CONTINUE;
            }
        );
    }

    _readProcessMemory() {
        try {
            const [ok, contents] = GLib.file_get_contents('/proc/self/status');
            if (!ok)
                return null;
            return parseProcessMemory(new TextDecoder().decode(contents));
        } catch {
            return null;
        }
    }

    _stopPerformanceMonitoring() {
        if (this._performanceTimer) {
            GLib.Source.remove(this._performanceTimer);
            this._performanceTimer = null;
        }
        if (this._performanceSignalId) {
            global.stage.disconnect(this._performanceSignalId);
            this._performanceSignalId = null;
        }
        this._performance = null;
    }

    GetPerformance() {
        const started = GLib.get_monotonic_time();
        const snapshot = buildActorTree(global.stage);
        const duration = GLib.get_monotonic_time() - started;
        const history = this._performance?.history || [];
        return JSON.stringify({
            sampledAt: GLib.get_real_time(),
            captureDurationUs: duration,
            actorCount: snapshot.actorCount,
            truncated: snapshot.truncated,
            incomplete: snapshot.incomplete,
            stage: {
                width: global.stage.width,
                height: global.stage.height,
            },
            monitorCount: Main.layoutManager.monitors.length,
            renderedFrames: this._performance?.renderedFrames || 0,
            framesPerSecond: history.at(-1)?.framesPerSecond ?? null,
            residentMemoryBytes: this._performance?.memory?.residentBytes ?? null,
            peakResidentMemoryBytes: this._performance?.memory?.peakResidentBytes ?? null,
            performanceHistory: history,
        });
    }

    disable() {
        this._finishActorPicker({ status: 'cancelled' });
        this._clearActorHighlight();
        this._stopPerformanceMonitoring();
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
