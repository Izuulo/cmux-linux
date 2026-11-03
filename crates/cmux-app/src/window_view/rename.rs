//! Inline workspace rename (plan D1): the key-to-text state machine and the input view.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameKey<'a> {
    Char(&'a str),
    Backspace,
    Enter,
    Escape,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenameOutcome {
    Editing,
    Commit(String),
    Cancel,
}

pub(crate) struct RenameState {
    pub text: String,
    /// The prefill is "selected": the next edit replaces it whole.
    pub replace_on_type: bool,
}

impl RenameState {
    pub fn new(current: &str) -> Self {
        Self {
            text: current.to_string(),
            replace_on_type: true,
        }
    }

    pub fn apply(&mut self, key: RenameKey) -> RenameOutcome {
        match key {
            RenameKey::Char(c) => {
                if std::mem::take(&mut self.replace_on_type) {
                    self.text.clear();
                }
                self.text.push_str(c);
            }
            RenameKey::Backspace => {
                if std::mem::take(&mut self.replace_on_type) {
                    self.text.clear();
                } else {
                    self.text.pop();
                }
            }
            RenameKey::Enter => return RenameOutcome::Commit(self.text.clone()),
            RenameKey::Escape => return RenameOutcome::Cancel,
            RenameKey::Other => {}
        }
        RenameOutcome::Editing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_char_replaces_then_appends() {
        let mut s = RenameState::new("Terminal");
        assert_eq!(s.apply(RenameKey::Char("a")), RenameOutcome::Editing);
        assert_eq!(s.text, "a");
        s.apply(RenameKey::Char("pi"));
        assert_eq!(s.text, "api");
        s.apply(RenameKey::Backspace);
        assert_eq!(s.text, "ap");
        assert_eq!(
            s.apply(RenameKey::Enter),
            RenameOutcome::Commit("ap".into())
        );
    }

    #[test]
    fn backspace_first_clears_and_escape_cancels() {
        let mut s = RenameState::new("é名");
        s.apply(RenameKey::Backspace);
        assert_eq!(s.text, "", "preselected text is deleted whole");
        let mut s = RenameState::new("x");
        s.replace_on_type = false;
        s.text = "é名".into();
        s.apply(RenameKey::Backspace);
        assert_eq!(s.text, "é", "backspace removes one char, not one byte");
        assert_eq!(s.apply(RenameKey::Escape), RenameOutcome::Cancel);
    }
}
