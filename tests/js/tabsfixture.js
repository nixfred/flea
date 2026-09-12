.pragma library

// The stubs the tab suite's newer cases share, kept here the way filterfixture.js keeps the
// filter suite's: tests/js/tabs.js sits at its file budget, and three cases needed the same
// two-tab shape spelled out.

// A pane standing in a search's results, which sets pane.path to the scope it walked and keeps the
// directory the operator was in on searchFrom.
function searching(p, from) {
    p.searchMode = "results"
    p.searchFrom = from
    return p
}

// Two tabs with the second current, so selectAt(0) and closeAt(1) both land on the first.
function twoTabs(p, a, b, extra) {
    var first = { path: a, history: [], cursorIndex: 4, viewMode: "list", showHidden: false,
                  selected: [], sortBy: "name", sortDesc: false }
    for (var k in (extra || {})) first[k] = extra[k]
    p.tabs = { items: [first, { path: b }], index: 1,
               pendingCursor: -1, pendingSortBy: "", pendingSortDesc: false }
    return p
}

// A pane searching from `from`, with a tab on `scope` to switch to or close onto.
function searchingOnScope(p, scope, from) {
    return twoTabs(searching(p, from), scope, from)
}

// Both tabs on one directory, the hidden one holding a selection made on listing 7. order is what
// the hidden tab recorded, so "size" is the switch that has to re-sort and "name" the one that does not.
function twin(p, order) {
    twoTabs(p, "/tmp/same", "/tmp/same", { selected: [2, 3], listed: 7, sortBy: order })
    p.backend.listRequests = 7
    p.thumbState = "warm"
    p.dirSizeState = "warm"
    p.selection.toggle(2)
    p.selection.toggle(3)
    return p
}
