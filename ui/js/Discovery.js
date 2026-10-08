.pragma library

.import "Mounts.js" as Mounts

// LAN hosts come from Avahi, which answers what is advertising a filesystem right now. Only the
// five service types below become rows, because a host that advertises nothing mountable is a
// name, not a place. Rows carry kind "share" with mounted false for the reason in Tailnet.js.
// Ported 2026-10-08 from the fork's feat/network-productivity branch.

var TYPES = {
    "_ssh._tcp": { scheme: "sftp", defaultPort: 22 },
    "_smb._tcp": { scheme: "smb", defaultPort: 445 },
    "_nfs._tcp": { scheme: "nfs", defaultPort: 2049 },
    "_webdav._tcp": { scheme: "dav", defaultPort: 80 },
    "_webdavs._tcp": { scheme: "davs", defaultPort: 443 }
}

// avahi-browse -artp emits semicolon-separated resolved rows beginning with "=". Duplicate rows
// collapse by normalized URI; unresolved, removal, malformed and unsupported rows are ignored.
function parse(body) {
    var lines = String(body || "").split("\n")
    var out = []
    var seen = {}
    for (var i = 0; i < lines.length; i++) {
        var fields = splitLine(lines[i])
        if (fields.length < 9 || fields[0] !== "=")
            continue
        var spec = TYPES[fields[4]]
        if (!spec)
            continue
        var host = cleanHost(fields[6] || fields[7])
        if (host.length === 0)
            continue
        var port = /^\d+$/.test(fields[8]) ? Number(fields[8]) : spec.defaultPort
        var uri = spec.scheme + "://" + hostForUri(host)
        // A default port is noise in a URI and would also split one host into two rows.
        if (port !== spec.defaultPort)
            uri += ":" + port
        uri = Mounts.normalize(uri + "/")
        if (seen[uri])
            continue
        seen[uri] = true
        out.push({ path: "", label: fields[3] || host, group: "network", kind: "share",
                   uri: uri, mounted: false, glyph: "server", origin: "lan", health: "online",
                   address: host, taildrop: false, mac: txtMac(fields.slice(9)) })
    }
    out.sort(function (a, b) { return a.label.localeCompare(b.label) })
    return out
}

// What the rail shows: the places the operator already has, then anything discovery found that is
// not already one of them. Saved places and live mounts always win, so a discovered row can never
// shadow a bookmark's own label, nor offer a second row for a share that is already mounted.
function merge(saved, discovered) {
    var out = (saved || []).slice()
    var seen = {}
    for (var i = 0; i < out.length; i++) {
        var key = keyOf(out[i])
        if (key.length > 0)
            seen[key] = true
    }
    var rows = discovered || []
    for (var j = 0; j < rows.length; j++) {
        var rowKey = keyOf(rows[j])
        if (rowKey.length === 0 || seen[rowKey])
            continue
        seen[rowKey] = true
        out.push(rows[j])
    }
    return out
}

function keyOf(entry) {
    if (!entry || !entry.uri)
        return ""
    return Mounts.normalize(String(entry.uri))
}

function splitLine(line) {
    var fields = String(line || "").split(";")
    for (var i = 0; i < fields.length; i++)
        fields[i] = unescapeField(fields[i])
    return fields
}

function unescapeField(value) {
    return String(value || "").replace(/\\(\d{3})/g, function (_, digits) {
        return String.fromCharCode(Number(digits))
    }).replace(/\\\\/g, "\\")
}

// A host name reaches a URI, so anything outside this set is dropped rather than quoted.
function cleanHost(value) {
    var host = String(value || "").trim().replace(/\.$/, "")
    return /^[A-Za-z0-9._:%-]+$/.test(host) ? host : ""
}

function hostForUri(host) {
    return host.indexOf(":") >= 0 && host.charAt(0) !== "[" ? "[" + host + "]" : host
}

function txtMac(fields) {
    var text = fields.join(" ")
    var m = text.match(/(?:^|[ "'])mac=([0-9a-f]{2}(?::[0-9a-f]{2}){5})(?:$|[ "'])/i)
    return m ? m[1].toLowerCase() : ""
}
