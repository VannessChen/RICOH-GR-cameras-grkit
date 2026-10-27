//! The languages of GRKit: of the menu text, the module text, the site, and
//! the installation guides.

use anyhow::{Context, Result};

/// A language of the menu text, the module text, and the installation guide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    SimplifiedChinese,
    TraditionalChinese,
    Japanese,
}

impl Language {
    pub const ALL: [Language; 4] = [
        Language::English,
        Language::SimplifiedChinese,
        Language::TraditionalChinese,
        Language::Japanese,
    ];

    /// Parses a language tag of the site: `en`, `zh-Hans`, `zh-Hant`, or `ja`.
    pub fn parse(code: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
            .with_context(|| format!("Unknown language {code:?}. Use en, zh-Hans, zh-Hant, or ja"))
    }

    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::SimplifiedChinese => "zh-Hans",
            Language::TraditionalChinese => "zh-Hant",
            Language::Japanese => "ja",
        }
    }
}
