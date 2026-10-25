//! The camera menu text in each menu language, and the label table that the
//! installer copies after the picture scratch (`runtime/abi.h`).
//!
//! The menu shows the language of the camera if it is a menu language, and
//! English otherwise. Effect names stay as their definitions give them.

use anyhow::{ensure, Result};

use crate::packaging::Language;

/// The size of the text area after the picture scratch.
pub const TEXT_SIZE: usize = 24576;
/// Where the Image Control summaries start in the text area. The label table
/// must end before it.
pub const TEXT_SUMMARY: usize = 8192;
/// The last 16 bytes of the text area: the parameter page title of a GRKit
/// session. Words: the page, its vtable, then the descriptor of the effect name.
pub const TEXT_PARAMETER_TITLE: usize = TEXT_SIZE - 16;
/// The 384 bytes before the parameter page title: the native settings page of
/// a GRKit session (`runtime/menu.c`).
pub const TEXT_SETTINGS: usize = TEXT_PARAMETER_TITLE - 384;
/// The units of the longest string with its NUL, and of each summary slot.
pub const SUMMARY_UNITS: usize = 32;
/// The number of strings in each language (`GRKIT_TEXT_STRINGS`).
pub const STRINGS: usize = 18;
/// The longest effect name in a record.
const NAME_UNITS: usize = 16;

/// The words of one menu language. Each list follows the rows of its module
/// after Off. The Chinese and Japanese words reuse the terms of the camera
/// menus where they have them (影像控制, 关闭/關閉, 弱/中/强, 颗粒感/粒紋感;
/// イメージコントロール, オフ, 粒状感, グリッド, 9分割, 画面中央, 弱/中/強).
/// The Date Imprint names are those of the GR II menu (Date Imprint, 加印日期摄影,
/// 加印日期攝像, 日付入れ撮影 in its firmware).
/// The camera's Chinese and Japanese fonts draw each of these characters only
/// while the camera language is that language, which is when the menu shows
/// them.
struct Words {
    /// Between a module name and its value in a summary.
    separator: &'static str,
    image_control: &'static str,
    /// Before the version in the About row.
    version: &'static str,
    off: &'static str,
    /// The Modules page, which turns modules on or off and orders them.
    modules: &'static str,
    /// The Modules summary after a change that applies at the next start.
    restart: &'static str,
    safe_mode: &'static str,
    /// The row that turns a module on or off, and its On value.
    load: &'static str,
    on: &'static str,
    /// The row that gives the place of a module in the GRKit root.
    position: &'static str,
    /// The Effects page, which turns effects on or off and orders them.
    effects: &'static str,
    /// The row that moves an effect, and its two values.
    order: &'static str,
    move_up: &'static str,
    move_down: &'static str,
}

const ENGLISH: Words = Words {
    separator: ": ",
    image_control: "Image Control",
    version: "Version",
    off: "Off",
    modules: "Modules",
    restart: "Restart to apply",
    safe_mode: "Safe mode",
    load: "Load",
    on: "On",
    position: "Position",
    effects: "Effects",
    order: "Order",
    move_up: "Move up",
    move_down: "Move down",
};

const SIMPLIFIED_CHINESE: Words = Words {
    separator: "：",
    image_control: "影像控制",
    version: "版本",
    off: "关闭",
    modules: "模块",
    restart: "重新开机后生效",
    safe_mode: "安全模式",
    load: "加载",
    on: "开",
    position: "位置",
    effects: "效果",
    order: "顺序",
    move_up: "上移",
    move_down: "下移",
};

const TRADITIONAL_CHINESE: Words = Words {
    separator: "：",
    image_control: "影像控制",
    version: "版本",
    off: "關閉",
    modules: "模組",
    restart: "重新開機後生效",
    safe_mode: "安全模式",
    load: "載入",
    on: "開",
    position: "位置",
    effects: "效果",
    order: "順序",
    move_up: "上移",
    move_down: "下移",
};

const JAPANESE: Words = Words {
    // A full-width colon would push "イメージコントロール：" and a 13-character
    // effect name under the icon column. The half-width colon keeps them clear.
    separator: ":",
    image_control: "イメージコントロール",
    version: "バージョン",
    off: "オフ",
    modules: "モジュール",
    restart: "再起動後に反映",
    safe_mode: "セーフモード",
    load: "読み込み",
    on: "オン",
    position: "位置",
    effects: "エフェクト",
    order: "順番",
    move_up: "上へ",
    move_down: "下へ",
};

/// The text between a page title and its value in a root row summary, such
/// as "Guides: Thirds".
pub fn separator(language: Language) -> &'static str {
    words(language).separator
}

fn words(language: Language) -> &'static Words {
    match language {
        Language::English => &ENGLISH,
        Language::SimplifiedChinese => &SIMPLIFIED_CHINESE,
        Language::TraditionalChinese => &TRADITIONAL_CHINESE,
        Language::Japanese => &JAPANESE,
    }
}

/// A run of consecutive string IDs. `runtime/abi.h` defines `TEXT_<name>` as
/// the first ID of each run.
pub struct Run {
    pub name: &'static str,
    pub count: usize,
    pub doc: &'static str,
}

const fn run(name: &'static str, count: usize, doc: &'static str) -> Run {
    Run { name, count, doc }
}

/// The string IDs in table order.
pub const RUNS: &[Run] = &[
    run("IMAGE_CONTROL", 1, ""),
    run("OFF", 1, ""),
    run("IMAGE_OFF_SUMMARY", 1, ""),
    run(
        "IMAGE_PREFIX",
        1,
        "The Image Control summary before an effect name, for the installer.",
    ),
    run("VERSION", 1, "The About row: the GRKit version."),
    run("MODULES", 1, "The title of the Modules page."),
    run(
        "MODULES_SUMMARY",
        3,
        "The Modules root row: no change, a change for the next start, safe mode.",
    ),
    run("LOAD", 1, "The settings row that turns a module on or off."),
    run("ON", 1, ""),
    run(
        "POSITION",
        1,
        "The settings row of the place of a module in the root.",
    ),
    run("EFFECTS", 1, "The title of the Effects page."),
    run(
        "EFFECTS_SUMMARY",
        2,
        "The last row of Image Control: no change, a change for the next start.",
    ),
    run("ORDER", 1, "The settings row that moves an effect."),
    run("MOVE_UP", 2, "Move up, then Move down."),
];

/// The first string ID of each run, in table order.
pub fn ids() -> impl Iterator<Item = (&'static Run, usize)> {
    RUNS.iter().scan(0, |next, run| {
        let id = *next;
        *next += run.count;
        Some((run, id))
    })
}

/// The first string ID of a run.
fn id(name: &str) -> usize {
    ids()
        .find(|(run, _)| run.name == name)
        .map(|(_, id)| id)
        .expect("a menu text run")
}

/// The strings of one run in one language. `version` is the GRKit version.
fn run_strings(w: &Words, name: &str, version: &str) -> Vec<String> {
    let summary = |module: &str, value: &str| format!("{module}{}{value}", w.separator);
    let words = |values: &[&str]| values.iter().map(|value| value.to_string()).collect();
    match name {
        "IMAGE_CONTROL" => words(&[w.image_control]),
        "OFF" => words(&[w.off]),
        "IMAGE_OFF_SUMMARY" => vec![summary(w.image_control, w.off)],
        "IMAGE_PREFIX" => vec![format!("{}{}", w.image_control, w.separator)],
        "VERSION" => vec![format!("{} {version}", w.version)],
        "MODULES" => words(&[w.modules]),
        "MODULES_SUMMARY" => vec![
            w.modules.to_string(),
            summary(w.modules, w.restart),
            summary(w.modules, w.safe_mode),
        ],
        "LOAD" => words(&[w.load]),
        "ON" => words(&[w.on]),
        "POSITION" => words(&[w.position]),
        "EFFECTS" => words(&[w.effects]),
        "EFFECTS_SUMMARY" => vec![w.effects.to_string(), summary(w.effects, w.restart)],
        "ORDER" => words(&[w.order]),
        "MOVE_UP" => words(&[w.move_up, w.move_down]),
        _ => unreachable!("Unknown menu text run {name}"),
    }
}

/// The strings of one language in the ID order of `runtime/abi.h`. `version`
/// is the GRKit version that the About row shows.
pub fn strings(language: Language, version: &str) -> Vec<String> {
    let w = words(language);
    RUNS.iter()
        .flat_map(|run| run_strings(w, run.name, version))
        .collect()
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The label table: for each language, `STRINGS` u16 byte offsets from the
/// table start, then the NUL-terminated UTF-16LE strings. `version` is the
/// GRKit version that the About row shows.
pub fn table(version: &str) -> Result<Vec<u8>> {
    ensure!(
        !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_graphic()),
        "The version {version:?} is not printable ASCII"
    );
    let languages = Language::ALL;
    let mut offsets = Vec::new();
    let mut strings_area: Vec<u16> = Vec::new();
    let strings_start = languages.len() * STRINGS * 2;
    for language in languages {
        let list = strings(language, version);
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
        let prefix = units(&list[id("IMAGE_PREFIX")]).len();
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

    const VERSION: &str = "0.0.0";

    #[test]
    fn the_table_fits_and_points_at_each_string() {
        let table = table(VERSION).unwrap();
        for (index, language) in Language::ALL.into_iter().enumerate() {
            for (id, expected) in strings(language, VERSION).iter().enumerate() {
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
    fn each_run_has_its_declared_number_of_strings() {
        for language in Language::ALL {
            for run in RUNS {
                assert_eq!(
                    run_strings(words(language), run.name, VERSION).len(),
                    run.count,
                    "{} in {language:?}",
                    run.name
                );
            }
        }
        assert_eq!(RUNS.iter().map(|run| run.count).sum::<usize>(), STRINGS);
    }

    #[test]
    fn english_keeps_the_menu_text_of_earlier_builds() {
        let english = strings(Language::English, VERSION);
        assert_eq!(english[id("IMAGE_CONTROL")], "Image Control");
        assert_eq!(english[id("OFF")], "Off");
        assert_eq!(english[id("IMAGE_OFF_SUMMARY")], "Image Control: Off");
        assert_eq!(english[id("IMAGE_PREFIX")], "Image Control: ");
    }

    #[test]
    fn the_about_row_shows_the_version_in_each_language() {
        assert_eq!(
            strings(Language::English, "1.2.3")[id("VERSION")],
            "Version 1.2.3"
        );
        assert_eq!(
            strings(Language::SimplifiedChinese, "1.2.3")[id("VERSION")],
            "版本 1.2.3"
        );
        assert_eq!(
            strings(Language::TraditionalChinese, "1.2.3")[id("VERSION")],
            "版本 1.2.3"
        );
        assert_eq!(
            strings(Language::Japanese, "1.2.3")[id("VERSION")],
            "バージョン 1.2.3"
        );
    }

    #[test]
    fn the_table_rejects_a_version_that_the_menu_cannot_show() {
        assert!(table("").is_err());
        assert!(table("1.0 beta").is_err());
        assert!(table("1.0.0-测试").is_err());
        assert!(table(&"9".repeat(SUMMARY_UNITS)).is_err());
    }
}
