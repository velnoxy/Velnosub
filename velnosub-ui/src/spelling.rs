use std::ops::Range;

pub(crate) fn words(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut cursor = 0;
    let mut in_override = false;
    let mut drawing = false;
    std::iter::from_fn(move || {
        while cursor < text.len() {
            let start = cursor;
            let tail = &text[start..];
            let first = tail.chars().next().unwrap();
            cursor += first.len_utf8();
            if first == '{' {
                in_override = true;
                if let Some(end) = tail.find('}') {
                    for tag in tail[1..end].split('\\').filter_map(|tag| tag.strip_prefix('p')) {
                        if let Some(level) = tag.chars().next().and_then(|level| level.to_digit(10)) {
                            drawing = level > 0;
                        }
                    }
                }
            } else if first == '}' {
                in_override = false;
            } else if !in_override && !drawing {
                if first == '\\' && tail[1..].starts_with(['N', 'n', 'h']) {
                    cursor += 1;
                } else if first.is_alphabetic() {
                    let rest = text[cursor..].char_indices()
                        .take_while(|(_, ch)| ch.is_alphabetic() || matches!(ch, '\'' | '’') ||
                            ('\u{0300}'..='\u{036f}').contains(ch));
                    for (_, ch) in rest { cursor += ch.len_utf8(); }
                    let end = text[start..cursor].trim_end_matches(['\'', '’']).len() + start;
                    return Some(start..end);
                }
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_ass_tags_drawing_and_escapes() {
        let text = r"{\i1}İyi\Nbugün\hdün {\p1}m n{\p0}don't";
        assert_eq!(words(text).map(|span| &text[span]).collect::<Vec<_>>(), ["İyi", "bugün", "dün", "don't"]);
    }

    #[test]
    fn loads_hunspell_rules() {
        let dict = spellbook::Dictionary::new("SET UTF-8\nSFX S Y 1\nSFX S 0 s .\n", "1\nbook/S\n").unwrap();
        assert!(dict.check("books"));
        assert!(!dict.check("bookx"));
    }
}
