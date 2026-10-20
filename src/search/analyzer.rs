//! Japanese-aware tokenization for the search index.
//!
//! Lindera (a Rust port of Kuromoji) does morphological analysis rather than
//! bigram splitting, so "京都" doesn't match "東京都", the dictionary form
//! "読む" matches "読んだ", and the decompose (search) mode splits compounds
//! ("関西国際空港" -> 関西/国際/空港). Non-CJK text is tokenized word by word
//! and lowercased. Each token then goes through the same filters as the
//! former Lucene analyzer: base form, width folding (NFKC), katakana stem
//! (trailing "ー" of long katakana words) and lowercasing. Punctuation never
//! becomes a token.

use lindera::dictionary::load_dictionary;
use lindera::mode::{Mode, Penalty};
use lindera::segmenter::Segmenter;
use std::borrow::Cow;
use std::sync::OnceLock;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};
use unicode_normalization::UnicodeNormalization;

pub const CONTENT_TOKENIZER: &str = "ja_content";
pub const FILENAME_TOKENIZER: &str = "ja_filename";

const KATAKANA_STEM_MIN_LENGTH: usize = 4;

/// The dictionary takes a moment to load, so it is loaded once per process.
fn segmenter() -> &'static Segmenter {
    static SEGMENTER: OnceLock<Segmenter> = OnceLock::new();
    SEGMENTER.get_or_init(|| {
        let dictionary = load_dictionary("embedded://ipadic").expect("embedded IPADIC dictionary");
        Segmenter::new(Mode::Decompose(Penalty::default()), dictionary, None)
    })
}

/// Loads the dictionary ahead of the first query.
pub fn warm_up() {
    let _ = segmenter();
}

#[derive(Clone)]
pub struct JapaneseTokenizer {
    /// File names are also split at case changes and between letters and digits
    /// ("QuarterlyReport2024" -> quarterly/report/2024).
    split_words: bool,
}

impl JapaneseTokenizer {
    pub fn content() -> Self {
        JapaneseTokenizer { split_words: false }
    }

    pub fn filename() -> Self {
        JapaneseTokenizer { split_words: true }
    }
}

pub struct VecTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for VecTokenStream {
    fn advance(&mut self) -> bool {
        self.index += 1;
        self.index <= self.tokens.len()
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index - 1]
    }
}

impl Tokenizer for JapaneseTokenizer {
    type TokenStream<'a> = VecTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> VecTokenStream {
        VecTokenStream { tokens: analyze(text, self.split_words), index: 0 }
    }
}

/// The tokens of `text` (offsets are byte offsets into `text`).
pub fn analyze(text: &str, split_words: bool) -> Vec<Token> {
    let mut out = Vec::new();
    let Ok(segments) = segmenter().segment(Cow::Borrowed(text)) else { return out };
    for mut seg in segments {
        let (start, end) = (seg.byte_start, seg.byte_end);
        let surface = seg.surface.to_string();
        if !surface.chars().any(char::is_alphanumeric) {
            continue;
        }
        let base = match seg.get_detail(6) {
            Some(b) if b != "*" && !b.is_empty() && is_japanese(&surface) => b.to_string(),
            _ => surface.clone(),
        };
        let pieces: Vec<(usize, usize, String)> = if split_words {
            split_word_parts(&surface)
                .into_iter()
                .map(|(s, e)| (start + s, start + e, surface[s..e].to_string()))
                .collect()
        } else {
            vec![(start, end, base)]
        };
        for (from, to, piece) in pieces {
            let term = normalize_term(&piece);
            if term.is_empty() {
                continue;
            }
            out.push(Token { offset_from: from, offset_to: to, position: out.len(), text: term, position_length: 1 });
        }
    }
    out
}

fn is_japanese(s: &str) -> bool {
    s.chars().any(|c| matches!(c as u32, 0x3040..=0x30ff | 0x3400..=0x9fff | 0xf900..=0xfaff | 0xff66..=0xff9f))
}

fn normalize_term(s: &str) -> String {
    let folded: String = s.nfkc().collect();
    let stemmed = katakana_stem(&folded);
    stemmed.to_lowercase()
}

/// "コンピューター" -> "コンピュータ": drops a trailing prolonged sound mark
/// from katakana words of at least 4 characters.
fn katakana_stem(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let all_katakana = chars.iter().all(|&c| ('\u{30a0}'..='\u{30ff}').contains(&c));
    if all_katakana && chars.len() >= KATAKANA_STEM_MIN_LENGTH && chars.last() == Some(&'ー') {
        chars[..chars.len() - 1].iter().collect()
    } else {
        s.to_string()
    }
}

#[derive(PartialEq, Clone, Copy)]
enum CharClass {
    Lower,
    Upper,
    Digit,
    Other,
}

fn class_of(c: char) -> CharClass {
    if c.is_ascii_digit() {
        CharClass::Digit
    } else if c.is_uppercase() {
        CharClass::Upper
    } else if c.is_lowercase() {
        CharClass::Lower
    } else {
        CharClass::Other
    }
}

/// Byte ranges of the word parts of a token, split like Lucene's
/// WordDelimiterGraphFilter: at lower->upper case changes ("fooBar"), before
/// the last capital of a capital run followed by lower case ("XMLFile" ->
/// XML/File), and between letters and digits.
fn split_word_parts(s: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut parts = Vec::new();
    let mut start = 0;
    for i in 1..chars.len() {
        let (prev, cur) = (class_of(chars[i - 1].1), class_of(chars[i].1));
        let next = chars.get(i + 1).map(|&(_, c)| class_of(c));
        let boundary = match (prev, cur) {
            (CharClass::Lower, CharClass::Upper) => true,
            (CharClass::Upper, CharClass::Upper) => next == Some(CharClass::Lower),
            (CharClass::Digit, CharClass::Lower | CharClass::Upper) => true,
            (CharClass::Lower | CharClass::Upper, CharClass::Digit) => true,
            _ => false,
        };
        if boundary {
            parts.push((start, chars[i].0));
            start = chars[i].0;
        }
    }
    parts.push((start, s.len()));
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(text: &str, split: bool) -> Vec<String> {
        analyze(text, split).into_iter().map(|t| t.text).collect()
    }

    #[test]
    fn japanese_morphological_analysis() {
        assert_eq!(terms("関西国際空港", false), vec!["関西", "国際", "空港"]);
        assert!(terms("本を読んだ", false).contains(&"読む".to_string()));
        assert!(!terms("東京都", false).contains(&"京都".to_string()));
        assert_eq!(terms("コンピューター", false), vec!["コンピュータ"]);
        assert_eq!(terms("ＡＢＣ", false), vec!["abc"]);
    }

    #[test]
    fn latin_words_and_punctuation() {
        assert_eq!(terms("Hello, World! Report.md", false), vec!["hello", "world", "report", "md"]);
    }

    #[test]
    fn filename_word_parts() {
        assert_eq!(terms("QuarterlyReport2024.pdf", true), vec!["quarterly", "report", "2024", "pdf"]);
        assert_eq!(terms("XMLFile", true), vec!["xml", "file"]);
    }

    #[test]
    fn offsets_point_into_the_text() {
        let text = "メモ: Rust入門";
        for t in analyze(text, false) {
            assert!(text.is_char_boundary(t.offset_from) && text.is_char_boundary(t.offset_to));
        }
    }
}
