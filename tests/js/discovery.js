.import "../../ui/js/Discovery.js" as Discovery

// The LAN half of NETWORK discovery, plus the merge that decides what the rail actually shows.
// Avahi's parseable rows are the input; no avahi-browse runs here. The merge checks are the ones
// that matter in daily use: a saved place must never be shadowed by a discovered row for the same
// location, or the rail would show one host twice and the operator's own label would lose.

function run(check) {
    var body = [
        '=;eth0;IPv4;Office\\032SSH;_ssh._tcp;local;workstation.local;192.168.1.8;22;"mac=aa:bb:cc:dd:ee:ff"',
        '=;eth0;IPv6;IPv6 SSH;_ssh._tcp;local;;fe80::1;22;',
        '=;eth0;IPv4;NAS;_smb._tcp;local;nas.local;192.168.1.9;445;',
        '=;eth0;IPv4;Secure Docs;_webdavs._tcp;local;docs.local;192.168.1.10;8443;',
        '+;eth0;IPv4;Unresolved;_ssh._tcp;local',
        '=;eth0;IPv4;Ignored;_printer._tcp;local;print.local;192.168.1.11;631;',
        '=;eth0;IPv4;Bad;_ssh._tcp;local;bad host;192.168.1.12;22;'
    ].join('\n')
    var rows = Discovery.parse(body)
    check("only supported resolved services survive", rows.length, 4)
    check("Avahi decimal escapes decode",
          rows.map(function (r) { return r.label }).indexOf("Office SSH") >= 0, true)
    check("default ports normalize away",
          rows.filter(function (r) { return r.label === "NAS" })[0].uri, "smb://nas.local/")
    check("non-default ports survive",
          rows.filter(function (r) { return r.label === "Secure Docs" })[0].uri, "davs://docs.local:8443/")
    check("IPv6 is bracketed",
          rows.filter(function (r) { return r.uri.indexOf("fe80") >= 0 })[0].uri, "sftp://[fe80::1]/")
    check("a discovered host carries the shape activate() mounts",
          rows[0].kind + "|" + rows[0].mounted + "|" + rows[0].group, "share|false|network")
    check("a TXT MAC is normalized",
          rows.filter(function (r) { return r.label === "Office SSH" })[0].mac, "aa:bb:cc:dd:ee:ff")
    check("empty and malformed discovery are harmless", Discovery.parse("garbage\n").length, 0)

    // The merge: saved places first and untouched, discovery only appending what is new.
    var saved = [{ label: "My NAS", uri: "smb://nas.local/", kind: "share", mounted: true, group: "network" }]
    var merged = Discovery.merge(saved, rows)
    check("a saved place keeps its own label and is not duplicated",
          merged.filter(function (e) { return e.uri === "smb://nas.local/" })
                .map(function (e) { return e.label }).join("|"), "My NAS")
    check("saved places lead and discovery follows", merged[0].label, "My NAS")
    check("everything discovered and not already saved is appended", merged.length, saved.length + rows.length - 1)
    check("a mounted saved place is never re-offered as unmounted",
          merged.filter(function (e) { return e.uri === "smb://nas.local/" })[0].mounted, true)
    check("merging nothing is the saved list itself", Discovery.merge(saved, []).length, 1)
    check("merging into nothing is just discovery", Discovery.merge([], rows).length, rows.length)
    check("rows without a uri never reach the rail", Discovery.merge([], [{ label: "x" }]).length, 0)
}
