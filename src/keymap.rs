use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    action::{Action, PanelFocus},
    screen::Mode,
};

/// Maps a raw key event to an `Action`, given the current mode and panel focus.
///
/// Pure function: no TUI, channel, or async dependency, so the keymap can be
/// exercised directly with plain `KeyCode`/`Mode`/`PanelFocus` values.
pub(crate) fn key_to_action(
    mode: Mode,
    focus: PanelFocus,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Option<Action> {
    if modifiers == KeyModifiers::CONTROL {
        match code {
            KeyCode::Char('z') => return Some(Action::Suspend),
            KeyCode::Char('c') => return Some(Action::Quit),
            _ => {}
        }
    }

    match code {
        KeyCode::Char('q') => return Some(Action::Quit),
        KeyCode::Char('?') => return Some(Action::ToggleHelp),
        _ => {}
    }

    // In Unified mode, navigation/selection only applies when the file tree
    // panel is focused; in the other modes it's always available.
    let file_tree_active = mode != Mode::Unified || focus == PanelFocus::FileTree;

    if file_tree_active {
        match code {
            KeyCode::Up | KeyCode::Char('k') => return Some(Action::CursorUp),
            KeyCode::Down | KeyCode::Char('j') => return Some(Action::CursorDown),
            KeyCode::Home => return Some(Action::CursorToTop),
            KeyCode::End => return Some(Action::CursorToBottom),
            KeyCode::Enter => return Some(Action::DirectoryOpenSelected),
            KeyCode::Right => return Some(Action::DirectoryExpand),
            KeyCode::Left => return Some(Action::DirectoryCollapse),
            KeyCode::Char(' ') => return Some(Action::SelectCurrent),
            KeyCode::Char('A') => return Some(Action::UnselectAll),
            KeyCode::Char('x') => return Some(Action::ToggleSkip),
            KeyCode::Char('n') => return Some(Action::JumpToNextNotRun),
            KeyCode::Char('S') => return Some(Action::SelectFromCursorToEnd),
            KeyCode::Char('r') => return Some(Action::ScriptRun(false)),
            KeyCode::Char('R') => return Some(Action::ScriptRun(true)),
            KeyCode::Char('C') => return Some(Action::CheckForChanges),
            _ => {}
        }
    }

    // Clear output is available in FileTree and ExecutionLog panels in
    // Unified mode, and unconditionally in the other modes.
    let clear_output_active =
        mode != Mode::Unified || matches!(focus, PanelFocus::FileTree | PanelFocus::ExecutionLog);
    if clear_output_active && code == KeyCode::Char('c') {
        return Some(Action::ClearOutput);
    }

    match (mode, code) {
        (Mode::FileChooser, KeyCode::Tab) => Some(Action::SwitchMode(Mode::ScriptRunner)),
        (Mode::ScriptRunner, KeyCode::Tab) => Some(Action::SwitchMode(Mode::FileChooser)),
        (Mode::Unified, KeyCode::Tab) => Some(Action::FocusNextPanel),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_and_help_work_everywhere() {
        for mode in [Mode::Unified, Mode::FileChooser, Mode::ScriptRunner] {
            for focus in [
                PanelFocus::FileTree,
                PanelFocus::ScriptPreview,
                PanelFocus::ExecutionLog,
            ] {
                assert_eq!(
                    key_to_action(mode, focus, KeyCode::Char('q'), KeyModifiers::NONE),
                    Some(Action::Quit)
                );
                assert_eq!(
                    key_to_action(mode, focus, KeyCode::Char('?'), KeyModifiers::NONE),
                    Some(Action::ToggleHelp)
                );
            }
        }
    }

    #[test]
    fn ctrl_c_and_ctrl_z_are_always_available() {
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::ScriptPreview,
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            key_to_action(
                Mode::FileChooser,
                PanelFocus::FileTree,
                KeyCode::Char('z'),
                KeyModifiers::CONTROL
            ),
            Some(Action::Suspend)
        );
    }

    #[test]
    fn unified_navigation_requires_file_tree_focus() {
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::FileTree,
                KeyCode::Down,
                KeyModifiers::NONE
            ),
            Some(Action::CursorDown)
        );
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::ScriptPreview,
                KeyCode::Down,
                KeyModifiers::NONE
            ),
            None
        );
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::ExecutionLog,
                KeyCode::Char('r'),
                KeyModifiers::NONE
            ),
            None
        );
    }

    #[test]
    fn file_chooser_and_script_runner_navigation_ignores_focus() {
        for mode in [Mode::FileChooser, Mode::ScriptRunner] {
            for focus in [
                PanelFocus::FileTree,
                PanelFocus::ScriptPreview,
                PanelFocus::ExecutionLog,
            ] {
                assert_eq!(
                    key_to_action(mode, focus, KeyCode::Up, KeyModifiers::NONE),
                    Some(Action::CursorUp)
                );
                assert_eq!(
                    key_to_action(mode, focus, KeyCode::Char('R'), KeyModifiers::NONE),
                    Some(Action::ScriptRun(true))
                );
                assert_eq!(
                    key_to_action(mode, focus, KeyCode::Char('C'), KeyModifiers::NONE),
                    Some(Action::CheckForChanges)
                );
            }
        }
    }

    #[test]
    fn clear_output_respects_unified_panel_restriction() {
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::FileTree,
                KeyCode::Char('c'),
                KeyModifiers::NONE
            ),
            Some(Action::ClearOutput)
        );
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::ExecutionLog,
                KeyCode::Char('c'),
                KeyModifiers::NONE
            ),
            Some(Action::ClearOutput)
        );
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::ScriptPreview,
                KeyCode::Char('c'),
                KeyModifiers::NONE
            ),
            None
        );
        assert_eq!(
            key_to_action(
                Mode::FileChooser,
                PanelFocus::ScriptPreview,
                KeyCode::Char('c'),
                KeyModifiers::NONE
            ),
            Some(Action::ClearOutput)
        );
    }

    #[test]
    fn tab_switches_or_focuses_depending_on_mode() {
        assert_eq!(
            key_to_action(
                Mode::FileChooser,
                PanelFocus::FileTree,
                KeyCode::Tab,
                KeyModifiers::NONE
            ),
            Some(Action::SwitchMode(Mode::ScriptRunner))
        );
        assert_eq!(
            key_to_action(
                Mode::ScriptRunner,
                PanelFocus::FileTree,
                KeyCode::Tab,
                KeyModifiers::NONE
            ),
            Some(Action::SwitchMode(Mode::FileChooser))
        );
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::FileTree,
                KeyCode::Tab,
                KeyModifiers::NONE
            ),
            Some(Action::FocusNextPanel)
        );
    }

    #[test]
    fn other_control_combos_fall_through_to_normal_matching() {
        // Only Ctrl+z/Ctrl+c are special-cased; any other Ctrl-modified key
        // is matched exactly like its unmodified counterpart, matching the
        // original inline match arms which never checked modifiers there.
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::FileTree,
                KeyCode::Up,
                KeyModifiers::CONTROL
            ),
            Some(Action::CursorUp)
        );
    }

    #[test]
    fn unmapped_key_returns_none() {
        assert_eq!(
            key_to_action(
                Mode::Unified,
                PanelFocus::FileTree,
                KeyCode::Char('z'),
                KeyModifiers::NONE
            ),
            None
        );
    }
}
