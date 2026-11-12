//! gpui actions for cmux's shortcuts and binding them from cmux-config's table.

use cmux_config::keybindings::{Binding, Keybindings};
use gpui::{Action, App, KeyBinding, KeyBindingContextPredicate, PlatformKeyboardMapper};

gpui::actions!(
    cmux,
    [
        Copy,
        Paste,
        SplitRight,
        SplitDown,
        ToggleSplitZoom,
        EqualizeSplits,
        ResizePaneLeft,
        ResizePaneRight,
        ResizePaneUp,
        ResizePaneDown,
        FocusLeft,
        FocusRight,
        FocusUp,
        FocusDown,
        FocusPreviousPane,
        FocusNextPane,
        NewSurface,
        CloseTab,
        CloseOtherTabsInPane,
        NextSurface,
        PrevSurface,
        MoveSurfaceLeft,
        MoveSurfaceRight,
        NewWorkspace,
        CloseWorkspace,
        RenameWorkspace,
        NextWorkspace,
        PrevWorkspace,
        MoveWorkspaceUp,
        MoveWorkspaceDown,
        ToggleSidebar,
        ShowNotifications,
        JumpToUnread,
        MarkAllNotificationsRead,
        ClearAllNotifications,
        ToggleRightSidebar,
        FocusRightSidebar,
        SwitchRightSidebarToFeed,
        SwitchRightSidebarToDock,
        NewWorkspaceGroup,
        ToggleWorkspaceGroupCollapsed,
        Quit,
    ]
);

/// cmux `selectSurfaceByNumber`: 1-8 pick that tab, 9 the last one.
#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = cmux, no_json)]
pub struct SelectSurfaceByNumber(pub u8);

/// cmux `selectWorkspaceByNumber`: 1-8 pick that workspace, 9 the last one.
#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = cmux, no_json)]
pub struct SelectWorkspaceByNumber(pub u8);

/// The gpui action for one config binding. `None` only if cmux-config lists an action this
/// table lacks (a test keeps them in sync).
pub fn action_for(b: &Binding) -> Option<Box<dyn Action>> {
    Some(match b.action {
        "copy" => Box::new(Copy),
        "paste" => Box::new(Paste),
        "splitRight" => Box::new(SplitRight),
        "splitDown" => Box::new(SplitDown),
        "toggleSplitZoom" => Box::new(ToggleSplitZoom),
        "equalizeSplits" => Box::new(EqualizeSplits),
        "resizePaneLeft" => Box::new(ResizePaneLeft),
        "resizePaneRight" => Box::new(ResizePaneRight),
        "resizePaneUp" => Box::new(ResizePaneUp),
        "resizePaneDown" => Box::new(ResizePaneDown),
        "focusLeft" => Box::new(FocusLeft),
        "focusRight" => Box::new(FocusRight),
        "focusUp" => Box::new(FocusUp),
        "focusDown" => Box::new(FocusDown),
        "focusPreviousPane" => Box::new(FocusPreviousPane),
        "focusNextPane" => Box::new(FocusNextPane),
        "newSurface" => Box::new(NewSurface),
        "closeTab" => Box::new(CloseTab),
        "closeOtherTabsInPane" => Box::new(CloseOtherTabsInPane),
        "nextSurface" => Box::new(NextSurface),
        "prevSurface" => Box::new(PrevSurface),
        "moveSurfaceLeft" => Box::new(MoveSurfaceLeft),
        "moveSurfaceRight" => Box::new(MoveSurfaceRight),
        "newTab" => Box::new(NewWorkspace),
        "closeWorkspace" => Box::new(CloseWorkspace),
        "renameWorkspace" => Box::new(RenameWorkspace),
        "nextSidebarTab" => Box::new(NextWorkspace),
        "prevSidebarTab" => Box::new(PrevWorkspace),
        "moveWorkspaceUp" => Box::new(MoveWorkspaceUp),
        "moveWorkspaceDown" => Box::new(MoveWorkspaceDown),
        "toggleSidebar" => Box::new(ToggleSidebar),
        "showNotifications" => Box::new(ShowNotifications),
        "jumpToUnread" => Box::new(JumpToUnread),
        "markAllNotificationsRead" => Box::new(MarkAllNotificationsRead),
        "clearAllNotifications" => Box::new(ClearAllNotifications),
        "toggleRightSidebar" => Box::new(ToggleRightSidebar),
        "focusRightSidebar" => Box::new(FocusRightSidebar),
        "switchRightSidebarToFeed" => Box::new(SwitchRightSidebarToFeed),
        "switchRightSidebarToDock" => Box::new(SwitchRightSidebarToDock),
        "newWorkspaceGroup" => Box::new(NewWorkspaceGroup),
        "toggleFocusedWorkspaceGroupCollapsed" => Box::new(ToggleWorkspaceGroupCollapsed),
        "quit" => Box::new(Quit),
        "selectWorkspaceByNumber" => Box::new(SelectWorkspaceByNumber(b.number?)),
        "selectSurfaceByNumber" => Box::new(SelectSurfaceByNumber(b.number?)),
        _ => return None,
    })
}

/// One config binding as a gpui `KeyBinding`: contextless, so one scheme decides precedence
/// (research-gpui.md §1.5), unless the config limits it to a key context. Never panics: a bad
/// keystroke is an `Err`.
pub fn key_binding(b: &Binding, mapper: &dyn PlatformKeyboardMapper) -> Result<KeyBinding, String> {
    let action = action_for(b).ok_or_else(|| format!("no handler for `{}`", b.action))?;
    let context = b
        .context
        .map(|c| KeyBindingContextPredicate::parse(c).map(Into::into))
        .transpose()
        .map_err(|e| format!("keybindings.{}: {e}", b.action))?;
    KeyBinding::load(&b.keystroke, action, context, false, None, mapper)
        .map_err(|e| format!("keybindings.{}: {e}", b.action))
}

/// Binds every entry; returns a warning per binding gpui rejected (the rest still bind).
pub fn bind(cx: &mut App, keybindings: &Keybindings) -> Vec<String> {
    let mapper = cx.keyboard_mapper().clone();
    let (ok, bad): (Vec<_>, Vec<_>) = keybindings
        .bindings
        .iter()
        .map(|b| key_binding(b, mapper.as_ref()))
        .partition(Result::is_ok);
    cx.bind_keys(ok.into_iter().map(Result::unwrap));
    bad.into_iter().map(|e| e.unwrap_err()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmux_config::keybindings::{DEFAULTS, keybindings};
    use gpui::{DummyKeyboardMapper, Keystroke, Modifiers};

    #[test]
    fn every_config_action_has_a_gpui_action() {
        for (name, _) in DEFAULTS {
            let b = Binding {
                action: name,
                keystroke: String::new(),
                number: Some(1),
                context: None,
            };
            assert!(action_for(&b).is_some(), "{name}");
        }
    }

    fn binding_for(action: &str) -> KeyBinding {
        let k = keybindings(None, "test");
        let b = k.bindings.iter().find(|b| b.action == action).unwrap();
        key_binding(b, &DummyKeyboardMapper).unwrap()
    }

    /// What gpui's Linux backend reports (platform/linux/platform.rs `from_xkb`).
    fn typed(
        platform: bool,
        control: bool,
        shift: bool,
        key: &str,
        key_char: Option<&str>,
    ) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                platform,
                control,
                shift,
                ..Default::default()
            },
            key: key.into(),
            key_char: key_char.map(Into::into),
        }
    }

    #[test]
    fn ctrl_4_shows_the_feed_only_in_the_right_sidebar() {
        let ctrl_4 = [typed(false, true, false, "4", None)];
        let feed = binding_for("switchRightSidebarToFeed");
        assert_eq!(feed.match_keystrokes(&ctrl_4), Some(false));
        let root = gpui::KeyContext::parse("Root").unwrap();
        let sidebar = gpui::KeyContext::parse("RightSidebar").unwrap();
        let pred = feed.predicate().unwrap();
        assert_eq!(pred.depth_of(std::slice::from_ref(&root)), None);
        assert!(pred.depth_of(&[root, sidebar]).is_some());
        // Tab 4 stays contextless. gpui 0.2.2 ranks a contextless binding as deepest, so inside
        // the sidebar both match; the Feed wins because tab 4's handler is on WorkspaceView (off
        // the sidebar's dispatch path) and its later DEFAULTS entry breaks the tie.
        let k = keybindings(None, "test");
        let tab = k
            .bindings
            .iter()
            .find(|b| b.action == "selectSurfaceByNumber" && b.keystroke == "ctrl-4")
            .unwrap();
        assert!(
            key_binding(tab, &DummyKeyboardMapper)
                .unwrap()
                .predicate()
                .is_none()
        );
    }

    #[test]
    fn the_default_table_parses_in_gpui() {
        for b in &keybindings(None, "test").bindings {
            assert!(key_binding(b, &DummyKeyboardMapper).is_ok(), "{b:?}");
        }
    }

    #[test]
    fn defaults_match_what_linux_reports() {
        // Super+Shift+] arrives as `}` with Shift cleared; Super+Shift+D keeps Shift.
        let next = binding_for("nextSurface");
        assert_eq!(
            next.match_keystrokes(&[typed(true, false, false, "}", Some("}"))]),
            Some(false)
        );
        let down = binding_for("splitDown");
        assert_eq!(
            down.match_keystrokes(&[typed(true, false, true, "d", Some("D"))]),
            Some(false)
        );
        let copy = binding_for("copy");
        assert_eq!(
            copy.match_keystrokes(&[typed(true, false, false, "c", Some("c"))]),
            Some(false)
        );
        assert_eq!(
            copy.match_keystrokes(&[typed(false, true, false, "c", None)]),
            None
        );
    }
}
