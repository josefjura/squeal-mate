use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::{Action, PanelFocus};

/// Maps a raw key event to an `Action`, given the current panel focus.
///
/// Pure function: no TUI, channel, or async dependency, so the keymap can be
/// exercised directly with plain `KeyCode`/`PanelFocus` values.
pub(crate) fn key_to_action(
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

    if code == KeyCode::Char('q') {
        return Some(Action::Quit);
    }

    if focus == PanelFocus::FileTree {
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

    if matches!(focus, PanelFocus::FileTree | PanelFocus::ExecutionLog)
        && code == KeyCode::Char('c')
    {
        return Some(Action::ClearOutput);
    }

    match code {
        KeyCode::Tab => Some(Action::FocusNextPanel),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_FOCUS: [PanelFocus; 3] = [
        PanelFocus::FileTree,
        PanelFocus::ScriptPreview,
        PanelFocus::ExecutionLog,
    ];

    #[test]
    fn quit_works_in_every_panel() {
        for focus in ALL_FOCUS {
            assert_eq!(
                key_to_action(focus, KeyCode::Char('q'), KeyModifiers::NONE),
                Some(Action::Quit)
            );
        }
    }

    #[test]
    fn ctrl_c_and_ctrl_z_are_always_available() {
        assert_eq!(
            key_to_action(
                PanelFocus::ScriptPreview,
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            ),
            Some(Action::Quit)
        );
        assert_eq!(
            key_to_action(
                PanelFocus::FileTree,
                KeyCode::Char('z'),
                KeyModifiers::CONTROL
            ),
            Some(Action::Suspend)
        );
    }

    #[test]
    fn navigation_requires_file_tree_focus() {
        assert_eq!(
            key_to_action(PanelFocus::FileTree, KeyCode::Down, KeyModifiers::NONE),
            Some(Action::CursorDown)
        );
        assert_eq!(
            key_to_action(PanelFocus::ScriptPreview, KeyCode::Down, KeyModifiers::NONE),
            None
        );
        assert_eq!(
            key_to_action(
                PanelFocus::ExecutionLog,
                KeyCode::Char('r'),
                KeyModifiers::NONE
            ),
            None
        );
    }

    #[test]
    fn clear_output_respects_panel_restriction() {
        assert_eq!(
            key_to_action(PanelFocus::FileTree, KeyCode::Char('c'), KeyModifiers::NONE),
            Some(Action::ClearOutput)
        );
        assert_eq!(
            key_to_action(
                PanelFocus::ExecutionLog,
                KeyCode::Char('c'),
                KeyModifiers::NONE
            ),
            Some(Action::ClearOutput)
        );
        assert_eq!(
            key_to_action(
                PanelFocus::ScriptPreview,
                KeyCode::Char('c'),
                KeyModifiers::NONE
            ),
            None
        );
    }

    #[test]
    fn tab_cycles_panel_focus() {
        for focus in ALL_FOCUS {
            assert_eq!(
                key_to_action(focus, KeyCode::Tab, KeyModifiers::NONE),
                Some(Action::FocusNextPanel)
            );
        }
    }

    #[test]
    fn other_control_combos_fall_through_to_normal_matching() {
        // Only Ctrl+z/Ctrl+c are special-cased; any other Ctrl-modified key
        // is matched exactly like its unmodified counterpart.
        assert_eq!(
            key_to_action(PanelFocus::FileTree, KeyCode::Up, KeyModifiers::CONTROL),
            Some(Action::CursorUp)
        );
    }

    #[test]
    fn unmapped_key_returns_none() {
        assert_eq!(
            key_to_action(PanelFocus::FileTree, KeyCode::Char('z'), KeyModifiers::NONE),
            None
        );
    }
}
