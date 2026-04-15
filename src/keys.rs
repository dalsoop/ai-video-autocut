use crossterm::event::KeyCode;

/// Nickel config의 문자열 "a", " ", "Enter" 등을 KeyCode로 비교
pub fn matches(code: KeyCode, binding: &str) -> bool {
    match (code, binding) {
        (KeyCode::Char(c), s) if s.chars().count() == 1 => {
            s.chars().next() == Some(c)
        }
        (KeyCode::Enter, "Enter") => true,
        (KeyCode::Esc, "Esc") => true,
        (KeyCode::Tab, "Tab") => true,
        (KeyCode::Backspace, "Backspace") => true,
        _ => false,
    }
}
