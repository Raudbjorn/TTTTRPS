//! Shared text input buffer with cursor management.
//!
//! Extracted from chat.rs for reuse in command palette and other input contexts.

/// A simple text input buffer with cursor positioning.
pub struct InputBuffer {
    content: String,
    cursor: usize,
}

impl InputBuffer {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor: 0,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.content[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.content.drain(prev..self.cursor);
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.cursor < self.content.len() {
            let next = self.content[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.content.len());
            self.content.drain(self.cursor..next);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor = self.content[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.content.len() {
            self.cursor = self.content[self.cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor + i)
                .unwrap_or(self.content.len());
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.content.len();
    }

    /// Take the content out, resetting the buffer.
    pub fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.content)
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_cursor() {
        let mut buf = InputBuffer::new();
        buf.insert_char('h');
        buf.insert_char('i');
        assert_eq!(buf.text(), "hi");
        assert_eq!(buf.cursor_position(), 2);
    }

    #[test]
    fn test_backspace() {
        let mut buf = InputBuffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.backspace();
        assert_eq!(buf.text(), "a");
        assert_eq!(buf.cursor_position(), 1);
    }

    #[test]
    fn test_movement() {
        let mut buf = InputBuffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.insert_char('c');
        buf.move_home();
        assert_eq!(buf.cursor_position(), 0);
        buf.move_end();
        assert_eq!(buf.cursor_position(), 3);
        buf.move_left();
        assert_eq!(buf.cursor_position(), 2);
        buf.move_right();
        assert_eq!(buf.cursor_position(), 3);
    }

    #[test]
    fn test_take_resets() {
        let mut buf = InputBuffer::new();
        buf.insert_char('x');
        let text = buf.take();
        assert_eq!(text, "x");
        assert!(buf.text().is_empty());
        assert_eq!(buf.cursor_position(), 0);
    }

    #[test]
    fn test_is_empty_trims() {
        let mut buf = InputBuffer::new();
        assert!(buf.is_empty());
        buf.insert_char(' ');
        assert!(buf.is_empty()); // whitespace-only is "empty"
        buf.insert_char('a');
        assert!(!buf.is_empty());
    }
}
