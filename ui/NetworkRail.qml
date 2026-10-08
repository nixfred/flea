import QtQuick
import "." as Flea
import "js/Discovery.js" as Discovery
import "js/Mounts.js" as Mounts

// NETWORK, with discovery folded in. This IS a NetworkMounts: the window-long host builds one of
// these instead, so every property, signal and function that file exposes is inherited unchanged
// and no caller knows the difference. Extending rather than wrapping is deliberate, because
// ui/NetworkMounts.qml and ui/Sidebar.qml both sit at the length tools/flea-file-budget records
// and may not grow by a line, and a hand-written façade would have to forward a surface of twenty
// members and silently break the rail the day one is missed.
//
// The merge is one-way: the saved places and live mounts NetworkMounts builds are the list, and
// discovery may only append hosts that are not already in it (see Discovery.merge). A discovered
// row is an unmounted kind "share", so the inherited activate() mounts and opens it.
Flea.NetworkMounts {
    id: rail

    property var discoveredEntries: discovery.entries

    // The list as NetworkMounts last built it, before anything was appended. Kept because the
    // merge must always run against that, never against its own previous output.
    property var _saved: []
    // entries is assigned inside this handler, which re-enters it; this is what ends the recursion.
    property bool _merging: false

    onEntriesChanged: {
        if (rail._merging) return
        rail._saved = rail.entries
        rail._apply()
    }

    onDiscoveredEntriesChanged: rail._apply()

    function _apply() {
        var merged = Discovery.merge(rail._saved, rail.discoveredEntries)
        // ui/NetworkMounts.qml only assigns entries when its rebuild differs, so that it does not
        // hand the rail a new model five seconds at a time for no change. Its own comparison can
        // never match here, because what it compares against is this merged list; so the same
        // guard is kept on this side, against the merge's own previous output.
        if (Mounts.sameEntries(rail.entries, merged))
            return
        rail._merging = true
        rail.entries = merged
        rail._merging = false
    }

    Flea.NetworkDiscovery {
        id: discovery
        // Discovery follows the same gate as the inherited mount listing: ui/Sidebar.qml counts a
        // loaded rail in through railArrived/railLeft, so with no rail on screen nothing polls and
        // a process nobody can see never runs. Deliberately NOT the listing's full condition: a
        // mount in flight needs a fresh gio table, it does not need the LAN browsed again.
        active: rail._rails > 0
    }
}
