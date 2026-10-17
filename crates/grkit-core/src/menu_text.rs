//! The camera menu text in each menu language, and the label table that the
//! installer copies after the picture scratch (`runtime/abi.h`).
//!
//! The menu shows the language of the camera if it is a menu language, and
//! English otherwise. Effect names stay as their definitions give them.

use anyhow::{ensure, Result};

use crate::packaging::Language;

/// The size of the text area after the picture scratch.
pub const TEXT_SIZE: usize = 8192;
/// Where the Image Control summaries start in the text area. The label table
/// must end before it.
pub const TEXT_SUMMARY: usize = 4096;
/// The units of the longest string with its NUL, and of each summary slot.
pub const SUMMARY_UNITS: usize = 32;
/// The number of strings in each language (`GRKIT_TEXT_STRINGS`).
pub const STRINGS: usize = 40;
/// The longest effect name in a record.
const NAME_UNITS: usize = 16;

/// The words of one menu language. Each list follows the rows of its module
/// after Off. The Chinese words reuse the terms of the camera manuals where
/// they have them (影像控制, 关闭/關閉, 弱/中/强, 颗粒感/粒紋感). 加印日期 comes
/// from the date imprint of earlier GR models. The camera's Chinese fonts draw
/// each of these characters only while the camera language is Chinese, which
/// is when the menu shows them.
struct Words {
    /// Between a module name and its value in a summary.
    separator: &'static str,
    image_control: &'static str,
    framing: &'static str,
    guides: &'static str,
    date_stamp: &'static str,
    grain: &'static str,
    off: &'static str,
    ratios: [&'static str; 3],
    /// The ratios in a summary, which has less room than a row.
    ratio_values: [&'static str; 3],
    guide_styles: [&'static str; 4],
    stamp_styles: [&'static str; 4],
    grain_levels: [&'static str; 3],
}

const ENGLISH: Words = Words {
    separator: ": ",
    image_control: "Image Control",
    framing: "Framing",
    guides: "Guides",
    date_stamp: "Date Stamp",
    grain: "Grain",
    off: "Off",
    ratios: ["XPan 65:24", "16:9", "4:3"],
    ratio_values: ["XPan", "16:9", "4:3"],
    guide_styles: ["Thirds", "Center", "Golden Ratio", "Diagonals"],
    stamp_styles: ["'YY M D", "'YY M D H:MM", "M D 'YY", "D M 'YY"],
    grain_levels: ["Weak", "Medium", "Strong"],
};

const SIMPLIFIED_CHINESE: Words = Words {
    separator: "：",
    image_control: "影像控制",
    framing: "画幅",
    guides: "辅助线",
    date_stamp: "加印日期",
    grain: "颗粒感",
    off: "关闭",
    ratios: ["XPan 65:24", "16:9", "4:3"],
    ratio_values: ["XPan", "16:9", "4:3"],
    guide_styles: ["三分法", "中心", "黄金分割", "对角线"],
    stamp_styles: ["'YY M D", "'YY M D H:MM", "M D 'YY", "D M 'YY"],
    grain_levels: ["弱", "中", "强"],
};

const TRADITIONAL_CHINESE: Words = Words {
    separator: "：",
    image_control: "影像控制",
    framing: "畫幅",
    guides: "輔助線",
    date_stamp: "加印日期",
    grain: "粒紋感",
    off: "關閉",
    ratios: ["XPan 65:24", "16:9", "4:3"],
    ratio_values: ["XPan", "16:9", "4:3"],
    guide_styles: ["三分法", "中心", "黃金分割", "對角線"],
    stamp_styles: ["'YY M D", "'YY M D H:MM", "M D 'YY", "D M 'YY"],
    grain_levels: ["弱", "中", "強"],
};

fn words(language: Language) -> &'static Words {
    match language {
        Language::English => &ENGLISH,
        Language::SimplifiedChinese => &SIMPLIFIED_CHINESE,
        Language::TraditionalChinese => &TRADITIONAL_CHINESE,
    }
}

/// The strings of one language in the ID order of `runtime/abi.h`.
pub fn strings(language: Language) -> Vec<String> {
    let w = words(language);
    let summary = |module: &str, value: &str| format!("{module}{}{value}", w.separator);
    let summaries = |module: &str, values: &[&str]| {
        std::iter::once(w.off)
            .chain(values.iter().copied())
            .map(|value| summary(module, value))
            .collect::<Vec<_>>()
    };
    let mut out: Vec<String> = [
        w.image_control,
        w.framing,
        w.guides,
        w.date_stamp,
        w.grain,
        w.off,
    ]
    .map(str::to_owned)
    .to_vec();
    out.extend(w.ratios.map(str::to_owned));
    out.extend(w.guide_styles.map(str::to_owned));
    out.extend(w.stamp_styles.map(str::to_owned));
    out.extend(w.grain_levels.map(str::to_owned));
    out.push(summary(w.image_control, w.off));
    out.extend(summaries(w.framing, &w.ratio_values));
    out.extend(summaries(w.guides, &w.guide_styles));
    out.extend(summaries(w.date_stamp, &w.stamp_styles));
    out.extend(summaries(w.grain, &w.grain_levels));
    out.push(format!("{}{}", w.image_control, w.separator));
    out
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The label table: for each language, `STRINGS` u16 byte offsets from the
/// table start, then the NUL-terminated UTF-16LE strings.
pub fn table() -> Result<Vec<u8>> {
    let languages = Language::ALL;
    let mut offsets = Vec::new();
    let mut strings_area: Vec<u16> = Vec::new();
    let strings_start = languages.len() * STRINGS * 2;
    for language in languages {
        let list = strings(language);
        ensure!(
            list.len() == STRINGS,
            "Menu text has {} strings",
            list.len()
        );
        for text in &list {
            let encoded = units(text);
            ensure!(
                encoded.len() < SUMMARY_UNITS,
                "Menu text is too long: {text:?}"
            );
            let offset = strings_start + strings_area.len() * 2;
            offsets.push(u16::try_from(offset)?);
            strings_area.extend(encoded);
            strings_area.push(0);
        }
        let prefix = units(list.last().unwrap()).len();
        ensure!(
            prefix + NAME_UNITS < SUMMARY_UNITS,
            "The Image Control prefix leaves no room for an effect name"
        );
    }
    let mut out: Vec<u8> = offsets.iter().flat_map(|v| v.to_le_bytes()).collect();
    out.extend(strings_area.iter().flat_map(|v| v.to_le_bytes()));
    ensure!(
        out.len() <= TEXT_SUMMARY,
        "Menu text table exceeds {TEXT_SUMMARY} bytes"
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_fits_and_points_at_each_string() {
        let table = table().unwrap();
        for (index, language) in Language::ALL.into_iter().enumerate() {
            for (id, expected) in strings(language).iter().enumerate() {
                let at = index * STRINGS + id;
                let offset = u16::from_le_bytes([table[at * 2], table[at * 2 + 1]]) as usize;
                let read: Vec<u16> = table[offset..]
                    .chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                    .take_while(|unit| *unit != 0)
                    .collect();
                assert_eq!(String::from_utf16(&read).unwrap(), *expected);
            }
        }
    }

    #[test]
    fn english_keeps_the_menu_text_of_earlier_builds() {
        let english = strings(Language::English);
        assert_eq!(english[1], "Framing");
        assert_eq!(english[22], "Framing: XPan");
        assert_eq!(english[20], "Image Control: Off");
        assert_eq!(english[39], "Image Control: ");
    }
}
