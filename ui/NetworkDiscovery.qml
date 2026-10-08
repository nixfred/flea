import QtQuick
import Quickshell
import Quickshell.Io
import "js/Discovery.js" as Discovery
import "js/Tailnet.js" as Tailnet

// What is out there that Flea could open, asked of the two things on this box that already know:
// "tailscale status --json" for the tailnet, "avahi-browse" for the LAN. Neither answer is a
// filesystem, so every row here is an unmounted kind "share"; ui/NetworkRail.qml folds them in
// behind the saved places, and ui/NetworkMounts.qml's own activate() mounts one on click.
//
// Both legs are polled and bounded exactly the way ui/MountListing.qml bounds its gio listing: a
// host that stops answering must not be able to hold the rail, so each leg carries its own
// deadline and the last good answer stands until a new one finishes.
Item {
    id: root

    // Discovery is cheap but not free, and nothing it finds changes second to second.
    readonly property int pollMs: 30000
    // avahi-browse -t ends by itself; this is the floor under a leg that does not.
    readonly property int timeoutMs: 8000
    // The C locale ui/NetworkMounts.qml pins on its own calls, for the same reason: the wording
    // of a refusal is read here, and no client locale reaches these tools.
    property var environment: ({ "LC_ALL": "C" })
    // The rail's own gate: the window-long host turns this off while no rail is loaded, so a
    // hidden rail costs no processes at all.
    property bool active: true
    // The local login that reaches an sftp:// authority; refused by Tailnet.safeUser if odd.
    property string user: Quickshell.env("USER") || ""

    // "missing", "daemon-stopped", "logged-out", "failed", "no-peers" or "ready". The rail does
    // not draw this yet; it exists so a later row can say why a tailnet is empty instead of
    // looking broken, which is the whole point of naming the state rather than the absence.
    property string tailnetState: "idle"
    property string tailnetMessage: ""
    property var tailnetPeers: []
    property var lanRows: []

    // Saved places win over discovery, so the merge against them happens in ui/NetworkRail.qml;
    // this one only de-duplicates the two sources against each other, tailnet first.
    readonly property var entries: Discovery.merge(Tailnet.entries(root.tailnetPeers, root.user), root.lanRows)

    property string _tailOut: ""
    property string _tailErr: ""
    property bool _tailTimedOut: false
    property string _lanOut: ""
    property bool _lanTimedOut: false

    function poll() {
        if (!root.active) return
        if (!tailProcess.running) {
            root._tailTimedOut = false
            root._tailOut = ""
            root._tailErr = ""
            tailProcess.running = true
            tailTimeout.restart()
        }
        if (!lanProcess.running) {
            root._lanTimedOut = false
            root._lanOut = ""
            lanProcess.running = true
            lanTimeout.restart()
        }
    }

    Timer {
        interval: root.pollMs
        running: root.active
        repeat: true
        triggeredOnStart: true
        onTriggered: root.poll()
    }

    Timer {
        id: tailTimeout
        interval: root.timeoutMs
        repeat: false
        onTriggered: {
            if (!tailProcess.running) return
            root._tailTimedOut = true
            tailProcess.running = false
        }
    }

    Process {
        id: tailProcess
        environment: root.environment
        command: ["tailscale", "status", "--json"]
        stdout: StdioCollector { id: tailOut; waitForEnd: true; onStreamFinished: if (!root._tailTimedOut) root._tailOut = tailOut.text }
        stderr: StdioCollector { id: tailErr; waitForEnd: true; onStreamFinished: if (!root._tailTimedOut) root._tailErr = tailErr.text }
        onExited: function (exitCode) {
            tailTimeout.stop()
            // A leg this timer ended collected nothing, and reading that as "no peers" would empty
            // the tailnet rows exactly when the daemon is the thing misbehaving.
            if (root._tailTimedOut) return
            var parsed = Tailnet.parseResult(exitCode, tailOut.text || root._tailOut, tailErr.text || root._tailErr)
            root.tailnetState = parsed.state
            root.tailnetMessage = parsed.message
            root.tailnetPeers = parsed.peers
        }
    }

    Timer {
        id: lanTimeout
        interval: root.timeoutMs
        repeat: false
        onTriggered: {
            if (!lanProcess.running) return
            root._lanTimedOut = true
            lanProcess.running = false
        }
    }

    Process {
        id: lanProcess
        environment: root.environment
        // -t ends the browse once the cache is exhausted, -r resolves, -p is the parseable form
        // ui/js/Discovery.js reads, -a asks for every type and the parser keeps the five it mounts.
        command: ["avahi-browse", "-artp"]
        stdout: StdioCollector { id: lanOut; waitForEnd: true; onStreamFinished: if (!root._lanTimedOut) root._lanOut = lanOut.text }
        onExited: function () {
            lanTimeout.stop()
            // An absent avahi-browse exits non-zero with nothing, which parses to no rows: the LAN
            // simply contributes nothing, and the tailnet half is unaffected.
            if (root._lanTimedOut) return
            root.lanRows = Discovery.parse(lanOut.text || root._lanOut)
        }
    }
}
