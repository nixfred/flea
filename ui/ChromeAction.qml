import QtQuick
import qs.Commons
import "." as Flea

// A control in a chrome strip, drawn under GM's 2026-09-11 rule: ink carries the role and never
// changes with state, and a wash behind the control carries the state. No frame: a hairline box
// costs two rules that go muddy as the scale drops, and Settings already said so with the segmented
// chooser's accent fill and no box around the group.
Item {
    id: root

    property string label: ""
    property string glyph: ""
    // plain is the neutral control, accent the one action a surface is asking for, error a permanent
    // deletion. The role is the ink, at every state; only the wash under it moves.
    property string role: "plain"
    property bool available: true
    // A primary control carries its wash at rest, because it is the thing the surface wants read
    // first. Every other control earns one only under the pointer or the keyboard.
    property bool primary: false

    signal activated()

    readonly property color ink: !root.available ? Theme.color.muted
        : root.role === "accent" ? Theme.color.accent
        : root.role === "error" ? Theme.color.error
        : Theme.color.foreground
    readonly property real wash: !root.available ? 0
        : (activeFocus || tap.pressed) ? Theme.washActive
        : hover.hovered ? Theme.washHover
        : root.primary ? Theme.washActive : 0

    enabled: root.available
    activeFocusOnTab: root.available
    implicitWidth: Math.max(Theme.hitMin, content.implicitWidth + 2 * Theme.spacing.gap)
    implicitHeight: Theme.chromeHeight

    Accessible.role: Accessible.Button
    Accessible.name: root.label.length > 0 ? root.label : root.glyph
    Accessible.onPressAction: if (root.available) root.activated()
    Keys.onReturnPressed: if (root.available) root.activated()
    Keys.onEnterPressed: if (root.available) root.activated()
    Keys.onSpacePressed: if (root.available) root.activated()

    // The wash, not the hit box: the item fills the strip so the press clears hitMin, while what is
    // drawn is the control's own caption line box centred in it.
    Rectangle {
        anchors.centerIn: parent
        width: parent.width
        height: Theme.chromeControlHeight
        color: Qt.alpha(root.ink, root.wash)
    }

    Row {
        id: content
        anchors.centerIn: parent
        spacing: Theme.spacing.gap / 2

        Flea.Glyph {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.glyph.length > 0
            width: Theme.chromeMarkSize
            height: Theme.chromeMarkSize
            name: root.glyph
            color: root.ink
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.label.length > 0
            text: root.label
            color: root.ink
            font.family: Theme.font.family
            font.pixelSize: Theme.font.caption
            textFormat: Text.PlainText
        }
    }

    HoverHandler {
        id: hover
        cursorShape: root.available ? Qt.PointingHandCursor : Qt.ArrowCursor
    }

    TapHandler {
        id: tap
        acceptedButtons: Qt.LeftButton
        gesturePolicy: TapHandler.ReleaseWithinBounds
        onTapped: if (root.available) root.activated()
    }
}
