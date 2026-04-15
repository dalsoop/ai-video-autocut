use unicode_width::UnicodeWidthChar;

pub fn truncate_right(s: &str, max_width: usize) -> String {
    let mut w = 0;
    let mut end = 0;
    for (i, c) in s.char_indices() {
        let cw = c.width().unwrap_or(0);
        if w + cw > max_width { break; }
        w += cw;
        end = i + c.len_utf8();
    }
    if end < s.len() {
        let mut t = s[..end].to_string();
        t.push('…');
        t
    } else { s.to_string() }
}

pub fn truncate_left(s: &str, max_width: usize) -> String {
    let mut widths: Vec<(usize, usize)> = Vec::new();
    let mut total = 0;
    for (i, c) in s.char_indices() {
        let cw = c.width().unwrap_or(0);
        widths.push((i, cw));
        total += cw;
    }
    if total <= max_width { return s.to_string(); }
    let mut w = 0;
    let mut start = 0;
    for (i, cw) in widths.iter().rev() {
        if w + cw > max_width - 1 { break; }
        w += cw;
        start = *i;
    }
    format!("…{}", &s[start..])
}

pub fn compute_viewport(cursor: usize, visible: usize, total: usize) -> usize {
    if total <= visible { return 0; }
    if cursor < visible / 2 { return 0; }
    if cursor >= total - visible / 2 { return total - visible; }
    cursor - visible / 2
}
