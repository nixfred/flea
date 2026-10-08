.import "../../ui/js/Tailnet.js" as Tailnet

// The tailnet half of NETWORK discovery. Every check here is the parser alone: no tailscale, no
// process, no network. What it proves is that a peer becomes the exact row shape
// ui/NetworkMounts.qml's activate() already knows how to mount, and that every way the CLI can
// refuse is named rather than collapsing into one silent empty rail.

function run(check) {
    var body = JSON.stringify({ BackendState: "Running", Self: { UserID: 7 }, Peer: {
        z: { HostName: "offline", Online: false, TailscaleIPs: ["100.1.2.5"], UserID: 7 },
        a: { HostName: "online", DNSName: "online.tail.ts.net.", Online: true, TaildropTarget: 1, UserID: 7 },
        m: { HostName: "relay.mullvad.ts.net", Online: true, TailscaleIPs: ["100.2.2.2"] }
    }})
    var ready = Tailnet.parseResult(0, body, "")
    check("running Tailscale is ready", ready.state, "ready")
    check("online peers sort first and relays disappear",
          ready.peers.map(function (p) { return p.label }).join("|"), "online|offline")
    check("DNS names lose only their terminal dot", ready.peers[0].address, "online.tail.ts.net")

    var entries = Tailnet.entries(ready.peers, "pi")
    check("a peer is an ordinary SFTP location", entries[0].uri, "sftp://pi@online.tail.ts.net/")
    // kind "share" with mounted false is the whole reason no second activation path was needed:
    // ui/NetworkMounts.qml activate() routes exactly this shape through openShare.
    check("a peer carries the shape activate() already mounts",
          entries[0].kind + "|" + entries[0].mounted + "|" + entries[0].group, "share|false|network")
    check("offline state survives into the rail", entries[1].health, "offline")
    check("an unsafe local user is never embedded",
          Tailnet.entries([ready.peers[0]], "bad@user")[0].uri, "sftp://online.tail.ts.net/")
    check("IPv6 addresses are bracketed",
          Tailnet.entries([{ id: "v6", label: "v6", address: "fd7a::1", online: true }], "")[0].uri, "sftp://[fd7a::1]/")

    check("a missing command is named", Tailnet.parseResult(127, "", "command not found").state, "missing")
    check("a stopped daemon is named",
          Tailnet.parseResult(1, "", "failed to connect to local tailscaled; not running").state, "daemon-stopped")
    check("stopped-daemon guidance names the recovery command",
          Tailnet.parseResult(1, "", "failed to connect to local tailscaled").message.indexOf("systemctl enable --now tailscaled") >= 0, true)
    check("a logged-out backend is named", Tailnet.parseResult(0, '{"BackendState":"NeedsLogin"}', "").state, "logged-out")
    check("a ready tailnet with no peers is distinct",
          Tailnet.parseResult(0, '{"BackendState":"Running","Peer":{}}', "").state, "no-peers")
    check("malformed status never throws", Tailnet.parseResult(0, "not json", "").state, "failed")
}
