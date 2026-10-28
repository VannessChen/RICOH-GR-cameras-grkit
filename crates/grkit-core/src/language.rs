//! The languages of GRKit: of the menu text, the module text, the site, and
//! the installation guides.

use anyhow::{Context, Result};

/// Each character that the camera's Korean menu font draws, in code order.
const KOREAN_FONT: &str = include_str!("korean_font.txt");

/// A language of the menu text, the module text, and the installation guide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
    TraditionalChinese,
    Japanese,
    Korean,
}

impl Language {
    pub const ALL: [Language; 5] = [
        Language::English,
        Language::SimplifiedChinese,
        Language::TraditionalChinese,
        Language::Japanese,
        Language::Korean,
    ];

    /// Parses a language tag of the site: `en`, `zh-Hans`, `zh-Hant`, `ja`, or `ko`.
    pub fn parse(code: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
            .with_context(|| {
                format!("Unknown language {code:?}. Use en, zh-Hans, zh-Hant, ja, or ko")
            })
    }

    /// The first character of `text` that the camera's font of this language
    /// cannot draw, which the camera then shows as "?". Only the Korean font is
    /// checked: it draws ASCII, the 2350 Hangul syllables of KS X 1001, and the
    /// other symbols of KS X 1001 that `korean_font.txt` lists (RT_KR.bin, the
    /// same on GR III and GR IV).
    pub fn undrawable(self, text: &str) -> Option<char> {
        match self {
            Language::Korean => text.chars().find(|&c| !KOREAN_FONT.trim_end().contains(c)),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::SimplifiedChinese => "zh-Hans",
            Language::TraditionalChinese => "zh-Hant",
            Language::Japanese => "ja",
            Language::Korean => "ko",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_korean_font_draws_ks_x_1001_and_ascii_only() {
        let font = KOREAN_FONT.trim_end();
        let hangul = font.chars().filter(|c| ('가'..='힣').contains(c)).count();
        assert_eq!((font.chars().count(), hangul), (3434, 2350));
        assert_eq!(Language::Korean.undrawable("GRKit 16:9 (꺼짐) 100%"), None);
        assert_eq!(Language::Korean.undrawable("똠"), Some('똠'));
        assert_eq!(Language::Korean.undrawable("A—B"), Some('—'));
        assert_eq!(Language::Japanese.undrawable("똠"), None);
    }
}
