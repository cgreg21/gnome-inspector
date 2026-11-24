import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

const BUS_NAME = 'io.github.cgreg21.GnomeInspector.Shell';
const OBJECT_PATH = '/io/github/cgreg21/GnomeInspector';
const XML = `
<node>
  <interface name="io.github.cgreg21.GnomeInspector">
    <method name="Ping"><arg type="s" direction="out"/></method>
    <method name="GetActorTree"><arg type="s" direction="out"/></method>
    <method name="GetStyleTree"><arg type="s" direction="out"/></method>
    <method name="GetActorComputedStyle"><arg type="s" direction="in"/><arg type="s" direction="out"/></method>
    <method name="GetThemeInfo"><arg type="s" direction="out"/></method>
    <method name="SetActorStyleClass"><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="out"/></method>
    <method name="SetActorInlineStyle"><arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="s" direction="out"/></method>
    <method name="GetDiagnostics"><arg type="s" direction="out"/></method>
    <method name="EvaluateGjs"><arg type="s" direction="in"/><arg type="s" direction="out"/></method>
    <method name="GetPerformance"><arg type="s" direction="out"/></method>
  </interface>
</node>`;

const root = {
    name: 'GNOME Shell Stage', type: 'Stage', visible: true, reactive: false,
    x: 0, y: 0, width: 1280, height: 800,
    children: [
        {
            name: 'Top Panel', type: 'StWidget', visible: true, reactive: true,
            x: 0, y: 0, width: 1280, height: 32,
            children: [
                { name: 'Activities', type: 'StButton', visible: true, reactive: true, x: 8, y: 0, width: 90, height: 32, children: [] },
                { name: 'Date Menu', type: 'PanelMenuButton', visible: true, reactive: true, x: 580, y: 0, width: 120, height: 32, children: [] },
                { name: 'Quick Settings', type: 'QuickSettingsMenu', visible: true, reactive: true, x: 1170, y: 0, width: 100, height: 32, children: [] }
            ]
        },
        {
            name: 'Overview', type: 'StWidget', visible: false, reactive: true,
            x: 0, y: 32, width: 1280, height: 768,
            children: [
                { name: 'Search Entry', type: 'StEntry', visible: false, reactive: true, x: 440, y: 100, width: 400, height: 48, children: [] },
                { name: 'Dash', type: 'Dash', visible: false, reactive: true, x: 400, y: 700, width: 480, height: 80, children: [] }
            ]
        },
        {
            name: 'Message Tray', type: 'MessageTray', visible: false, reactive: true,
            x: 0, y: 32, width: 1280, height: 768, children: []
        }
    ]
};

const styles = {
    name: 'GNOME Shell Stage', type: 'Stage', path: '', styleClass: '', pseudoClass: '', inlineStyle: '',
    children: [
        {
            name: 'Top Panel', type: 'StWidget', path: '0', styleClass: 'panel', pseudoClass: '', inlineStyle: '',
            children: [
                { name: 'Activities', type: 'StButton', path: '0/0', styleClass: 'panel-button', pseudoClass: 'hover', inlineStyle: 'padding: 0 8px;', children: [] },
                { name: 'Date Menu', type: 'PanelMenuButton', path: '0/1', styleClass: 'panel-button clock', pseudoClass: '', inlineStyle: '', children: [] },
                { name: 'Quick Settings', type: 'QuickSettingsMenu', path: '0/2', styleClass: 'panel-button', pseudoClass: '', inlineStyle: '', children: [] }
            ]
        },
        {
            name: 'Overview', type: 'StWidget', path: '1', styleClass: 'overview', pseudoClass: '', inlineStyle: '',
            children: [
                { name: 'Search Entry', type: 'StEntry', path: '1/0', styleClass: 'search-entry', pseudoClass: 'focus', inlineStyle: '', children: [] },
                { name: 'Dash', type: 'Dash', path: '1/1', styleClass: 'dash', pseudoClass: '', inlineStyle: '', children: [] }
            ]
        },
        { name: 'Message Tray', type: 'MessageTray', path: '2', styleClass: 'message-tray', pseudoClass: '', inlineStyle: '', children: [] }
    ]
};

const history = Array.from({ length: 8 }, (_, index) => ({
    sampledAt: Date.now() * 1000 - (7 - index) * 1000000,
    intervalUs: 1000000,
    renderedFrames: 58 + index,
    framesPerSecond: 58 + index
}));

class MockInspector {
    Ping() {
        return '51.1';
    }

    GetActorTree() {
        return JSON.stringify({ actorCount: 9, truncated: false, incomplete: false, root });
    }

    GetStyleTree() {
        return JSON.stringify({ actorCount: 9, truncated: false, root: styles });
    }

    GetActorComputedStyle(path) {
        return JSON.stringify({
            available: true,
            node: 'StWidget#panel.panel-button',
            elementId: path === '0/0' ? 'activities' : 'clock',
            elementClasses: ['panel-button', 'panel-button-hover'],
            pseudoClasses: ['hover'],
            font: 'Cantarell 11',
            width: 90, height: 32, minWidth: 0, maxWidth: -1,
            minHeight: 0, maxHeight: -1,
            horizontalPadding: 8, verticalPadding: 4, letterSpacing: 0
        });
    }

    GetThemeInfo() {
        return JSON.stringify({
            available: true,
            applicationStylesheet: '/usr/share/gnome-shell/theme/gnome-shell.css',
            themeStylesheet: '/usr/share/themes/Adwaita/gnome-shell/gnome-shell.css',
            defaultStylesheet: '/usr/share/gnome-shell/theme/gnome-shell.css',
            customStylesheets: []
        });
    }

    SetActorStyleClass(path, styleClass) {
        return JSON.stringify({ ok: true, path, styleClass });
    }

    SetActorInlineStyle(path, css) {
        return JSON.stringify({ ok: true, path, css });
    }

    GetDiagnostics() {
        return JSON.stringify({
            shellVersion: '51.1',
            sessionType: 'wayland',
            waylandDisplay: 'wayland-0',
            display: ':0',
            stage: { width: 1280, height: 800 },
            monitorCount: 1,
            extension: {
                uuid: 'gnome-inspector@cgreg21',
                version: 1,
                path: '/home/demo/.local/share/gnome-shell/extensions/gnome-inspector@cgreg21'
            }
        });
    }

    EvaluateGjs(source) {
        return JSON.stringify({
            input: source,
            result: ['Main', 'global', 'imports', 'St', 'Clutter']
        }, null, 2);
    }

    GetPerformance() {
        return JSON.stringify({
            sampledAt: Date.now() * 1000,
            captureDurationUs: 1240,
            actorCount: 9,
            truncated: false,
            incomplete: false,
            stage: { width: 1280, height: 800 },
            monitorCount: 1,
            renderedFrames: 4821,
            framesPerSecond: 60,
            residentMemoryBytes: 268435456,
            peakResidentMemoryBytes: 301989888,
            cpuPercent: 18.4,
            cpuTimeNs: 123456789000,
            threadCount: 14,
            voluntaryContextSwitches: 1200,
            involuntaryContextSwitches: 35,
            performanceHistory: history.map((sample, index) => ({
                ...sample,
                cpuPercent: index === 12 ? null : 12 + index * 0.8,
                cpuTimeNs: 110000000000 + index * 1000000000,
                threadCount: 14,
                voluntaryContextSwitches: 1180 + index * 20,
                involuntaryContextSwitches: 30 + index,
                residentMemoryBytes: index === 24 ? null : 260000000 + index * 1000000,
                peakResidentMemoryBytes: 301989888
            }))
        });
    }
}

let exported = null;
Gio.bus_own_name(
    Gio.BusType.SESSION,
    BUS_NAME,
    Gio.BusNameOwnerFlags.NONE,
    connection => {
        exported = Gio.DBusExportedObject.wrapJSObject(XML, new MockInspector());
        exported.export(connection, OBJECT_PATH);
    },
    null,
    () => {
        exported?.unexport();
        exported = null;
    }
);
GLib.MainLoop.new(null, false).run();
