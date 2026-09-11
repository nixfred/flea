import QtQuick
import qs.Commons
import "." as Flea

// A control in a chrome strip, drawn under GM's 2026-09-11 ruling. Two inks: a STRUCTURAL RULE
// recedes at the hairline, and a CONTROL FRAME advances in the control's own role. Drawing both in
// one ink, which is what ui/picker.qml's comment admits the boards did, makes a control's edge and a
// strip's edge the same line, and the control dissolves into the chrome as the scale drops. The
// frame carries the role, the wash inside it carries the state. ui/DialogButton.qml always did this.
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
    // A neutral control rests in muted, which is the role ThemeRoles.html gives an inactive control,
    // and an unavailable one stays there: the single case where a frame may recede, because an inert
    // control should. Everything else frames in the ink it is already written in.
    readonly property color frame: !root.available || root.role === "plain" ? Theme.color.muted : root.ink
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

    // The item fills the strip so the press clears hitMin, while what is DRAWN is the control's own
    // caption line box centred in it. That inset is what keeps this frame off the strip's own rule:
    // two lines that touch read as one line whatever colour they are.
    Rectangle {
        anchors.centerIn: parent
        width: parent.width
        height: Theme.chromeControlHeight
        color: Qt.alpha(root.ink, root.wash)
        border.width: Theme.spacing.hairline
        border.color: root.frame
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
