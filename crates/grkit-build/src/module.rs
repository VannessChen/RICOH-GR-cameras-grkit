//! Build a module file (`grkit_core::grm`) from C sources.
//!
//! A module is Thumb code that includes only `runtime/sdk/grkit.h`. It calls
//! the core through literal words (`-mlong-calls`), because the module memory
//! lies far from the core, out of BL range. It loads each address from a
//! literal word (`-mno-movt`). The installer therefore moves each reference
//! with one word: every kept relocation that is not PC-relative inside the code
//! is an `R_ARM_ABS32` word.

use crate::{
    arm::{defined_symbols, elf_module, undefined_symbols},
    components::path,
    toolchain::{run, Toolchain},
};
use anyhow::{bail, ensure, Context, Result};
use grkit_core::grm::{Import, Module, Segment, DATA_LINK};
use std::{fmt::Write, fs, path::Path};

/// The symbol of the module descriptor (`GRKIT_MODULE`).
const DESCRIPTOR: &str = "grkit_module";
/// Each import gets a distinct placeholder address in the link, so a word that
/// refers to an import is known by its value too.
const IMPORT_BASE: u32 = 0xf000_0000;
/// The stack frame limit of module code, which runs on the stacks of firmware
/// tasks. It is that of the core code on the same tasks (the picture code of
/// the framing component).
const FRAME_LIMIT: &str = "-Wframe-larger-than=1536";

/// Compile and link `sources` into the module `id`. `directory` receives the
/// objects; `sdk` holds `grkit.h`.
pub fn build(
    id: &str,
    version: u32,
    sources: &[&Path],
    sdk: &Path,
    directory: &Path,
    tools: &Toolchain,
) -> Result<Module> {
    build_with(id, version, sources, &[sdk], directory, tools)
}

/// `build` with more include folders.
fn build_with(
    id: &str,
    version: u32,
    sources: &[&Path],
    includes: &[&Path],
    directory: &Path,
    tools: &Toolchain,
) -> Result<Module> {
    ensure!(
        grkit_core::grm::valid_id(id),
        "Invalid module ID {id:?}: use 1 to 8 characters A-Z, 0-9, or _"
    );
    ensure!(!sources.is_empty(), "A module needs a source file");
    let mut objects = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let object = directory.join(format!("module-{index}.o"));
        let mut include_flags = Vec::new();
        for include in includes {
            include_flags.push("-I".to_owned());
            include_flags.push(path(include)?.to_owned());
        }
        run(
            &tools.clang,
            [
                "--target=arm-none-eabi",
                "-march=armv7-a",
                "-mthumb",
                "-Os",
                "-ffreestanding",
                "-fno-builtin",
                "-fno-common",
                "-fno-stack-protector",
                "-fno-unwind-tables",
                "-fno-asynchronous-unwind-tables",
                "-mlong-calls",
                "-mno-movt",
                FRAME_LIMIT,
                "-Werror=frame-larger-than",
                "-c",
                path(source)?,
                "-o",
                path(&object)?,
            ]
            .into_iter()
            .chain(include_flags.iter().map(String::as_str)),
        )?;
        objects.push(object);
    }
    // The imports: the symbols that no object of the module defines.
    let (mut imports, mut defined) = (
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
    );
    for object in &objects {
        let bytes = fs::read(object)?;
        imports.extend(undefined_symbols(&bytes)?);
        defined.extend(defined_symbols(&bytes)?);
    }
    imports.retain(|name| !defined.contains(name));
    // A compiler helper, such as a division, would be an import that the core
    // does not export.
    if let Some(name) = imports
        .iter()
        .find(|name| !grkit_core::grm::importable(name))
    {
        bail!("The module uses {name}, which the core does not export (runtime/sdk/grkit.h)");
    }
    let mut script = format!(
        "SECTIONS {{\n . = 0;\n .text : {{ *(.text*) *(.rodata*) *(.data.rel.ro*) . = ALIGN(4); }}\n . = {DATA_LINK:#x};\n .data : {{ *(.data*) . = ALIGN(4); }}\n .bss : {{ *(.bss*) *(COMMON) . = ALIGN(4); }}\n /DISCARD/ : {{ *(.ARM.exidx*) *(.ARM.extab*) *(.comment*) }}\n}}\n"
    );
    let placeholders: Vec<(String, u32)> = imports
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), IMPORT_BASE + index as u32 * 16))
        .collect();
    for (name, address) in &placeholders {
        writeln!(script, "{name} = {address:#x};")?;
    }
    let script_path = directory.join("module.ld");
    fs::write(&script_path, script)?;
    let elf = directory.join("module.elf");
    let mut arguments = vec![
        "--emit-relocs".to_owned(),
        "-T".to_owned(),
        path(&script_path)?.to_owned(),
        "-o".to_owned(),
        path(&elf)?.to_owned(),
    ];
    for object in &objects {
        arguments.push(path(object)?.to_owned());
    }
    run(&tools.lld, arguments.iter().map(String::as_str))?;
    convert(id, version, &fs::read(elf)?, &placeholders)
}

/// The module of a linked ELF file.
fn convert(id: &str, version: u32, elf: &[u8], placeholders: &[(String, u32)]) -> Result<Module> {
    let mut linked = elf_module(elf)?;
    let descriptor = match linked.symbols.get(DESCRIPTOR) {
        Some(&(Segment::Code, offset)) => offset,
        Some(_) => bail!("{DESCRIPTOR} must be constant: define it with GRKIT_MODULE"),
        None => bail!("The module has no {DESCRIPTOR}: define it with GRKIT_MODULE"),
    };
    let mut module = Module {
        id: id.to_owned(),
        version,
        bss_size: linked.bss_size,
        descriptor,
        ..Module::default()
    };
    for relocation in &linked.relocations {
        let word = matches!(relocation.kind, 2 | 38);
        let bytes = match relocation.segment {
            Segment::Code => &mut linked.code,
            Segment::Data => &mut linked.data,
        };
        let offset = relocation.offset as usize;
        if !word {
            // A PC-relative reference moves with the code, if both ends are in
            // it. R_ARM_NONE marks nothing.
            ensure!(
                relocation.kind == 0
                    || (relocation.segment == Segment::Code
                        && relocation.target == Some(Segment::Code)
                        && !matches!(relocation.kind, 43 | 44 | 47 | 48)),
                "Unsupported relocation type {} of {} in the module: build with -mlong-calls and -mno-movt",
                relocation.kind,
                relocation.symbol
            );
            continue;
        }
        ensure!(
            offset.is_multiple_of(4),
            "Unaligned word relocation of {}",
            relocation.symbol
        );
        if relocation.target.is_some() {
            match relocation.segment {
                Segment::Code => module.code_relocations.push(relocation.offset),
                Segment::Data => module.data_relocations.push(relocation.offset),
            }
            continue;
        }
        let (name, address) = placeholders
            .iter()
            .find(|(name, _)| *name == relocation.symbol)
            .with_context(|| format!("Unexpected absolute symbol {}", relocation.symbol))?;
        let value = u32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        ensure!(
            value == *address,
            "The module refers to {name} with an offset: an import must be a function or a whole object"
        );
        bytes[offset..offset + 4].fill(0);
        module.imports.push(Import {
            name: name.clone(),
            segment: relocation.segment,
            offset: relocation.offset,
        });
    }
    module.code = linked.code;
    module.data = linked.data;
    module.validate()?;
    Ok(module)
}

/// `module.toml`: what a module is, which `grkit module build` turns into its
/// descriptor (`GrkitModule` in `runtime/sdk/grkit.h`).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub version: u32,
    /// The default place in the GRKit root of ADJ and in the GRKit section
    /// of MENU: a smaller order first, then by ID (`grkit_core::grm`).
    #[serde(default = "default_order")]
    pub order: u32,
    /// The C sources, relative to the manifest. A module with only data,
    /// such as a page of a capability, has none.
    #[serde(default)]
    pub sources: Vec<String>,
    /// The 40 x 40 PNG tiles, relative to the manifest. A choice page shows
    /// tile n for value n.
    #[serde(default)]
    pub tiles: Vec<String>,
    /// Full names of settings of other modules or of the core, MODULE.KEY,
    /// with '?' first for an optional one.
    #[serde(default)]
    pub uses: Vec<String>,
    #[serde(default)]
    pub settings: Vec<SettingSpec>,
    pub page: Option<PageSpec>,
    #[serde(default)]
    pub layers: Vec<LayerSpec>,
    #[serde(default)]
    pub filters: Vec<FilterSpec>,
    pub events: Option<EventsSpec>,
}

fn default_order() -> u32 {
    grkit_core::grm::DEFAULT_ORDER
}

/// The event subscriber of a module (`GrkitSubscriber`).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsSpec {
    /// The C function `void handle(u32 event, u32 a, u32 b)`.
    pub handle: String,
    /// The events: `system_ready`, `setting_changed`.
    pub on: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingSpec {
    pub key: String,
    pub values: u32,
    #[serde(default)]
    pub saved: bool,
    #[serde(default)]
    pub public: bool,
}

/// A choice page of one setting: a key of `settings`, or a name of `uses`.
/// Its text is in `text/<language>.toml`.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpec {
    pub setting: String,
    /// Row 0 shows the Off icon instead of tile 0.
    #[serde(default)]
    pub off_row: bool,
    /// The rows of the native settings page: 1 to 5 keys of `settings`, the
    /// first the page setting. The text names each in `[settings.<KEY>]`.
    #[serde(default)]
    pub settings: Vec<String>,
    /// The settings page of the module in MENU has an Options row, which turns
    /// each value of the page on or off for ADJ; for a page of CORE.TONE, its
    /// effects (`GRKIT_CHOICE_OPTIONS`).
    #[serde(default)]
    pub options: bool,
}

/// A `picture.plane` filter.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterSpec {
    /// The C function `void apply(PictureSet *, u32 value)`.
    pub apply: String,
    /// The setting whose value the filter applies: a key of `settings`, or a
    /// name of `uses`.
    pub setting: String,
    /// `frame` or `mark` (runtime/sdk/grkit.h: GRKIT_STAGE_*).
    pub stage: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerSpec {
    /// The C function `void paint(const Surface *, u32 value)`.
    pub paint: String,
    /// The setting whose value the layer draws: a key of `settings`, or a
    /// name of `uses`. Without it, the value is 1.
    pub setting: Option<String>,
    pub order: u32,
    /// Off while DISP hides the native information.
    #[serde(default)]
    pub follow_disp: bool,
    /// On only while DISP shows the full information.
    #[serde(default)]
    pub full_disp: bool,
    /// Not on the ADJ canvas.
    #[serde(default)]
    pub live_only: bool,
}

/// `text/<language>.toml`: the text of the page in one menu language.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PageText {
    title: String,
    /// One label for each value of the page setting. A page whose rows the
    /// core gives (`CORE_ROWS`) has none.
    #[serde(default)]
    rows: Vec<String>,
    /// The value in each root row summary, `<title><separator><value>`, if it
    /// is not the label. A summary has less room than a row.
    summaries: Option<Vec<String>>,
    /// The rows of the native settings page (`PageSpec::settings`) by key.
    #[serde(default)]
    settings: std::collections::BTreeMap<String, RowText>,
}

/// The text of a row of the native settings page.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RowText {
    label: String,
    /// The text of each value. The page setting uses the page rows instead.
    values: Option<Vec<String>>,
}

/// The capabilities whose pages get their rows from the core
/// (`runtime/sdk/grkit.h`: GRKIT_CHOICE_CORE_ROWS).
const CORE_ROWS: [&str; 1] = ["CORE.TONE"];

/// The most UTF-16 units of one text (`runtime/module_link.h`, GRM_TEXT_UNITS).
const TEXT_UNITS: usize = 64;

fn c_name(name: &str) -> Result<&str> {
    ensure!(
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "{name:?} is not a C name"
    );
    Ok(name)
}

fn valid_key(key: &str) -> bool {
    (1..=15).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// A UTF-16 C array with its NUL.
fn utf16(text: &str) -> Result<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    ensure!(
        units.len() <= TEXT_UNITS,
        "The text {text:?} is longer than {TEXT_UNITS} UTF-16 units"
    );
    let mut out = String::from("{");
    for unit in units.iter().chain([&0]) {
        write!(out, "{unit},")?;
    }
    out.push('}');
    Ok(out)
}

/// Read `module.toml` in `directory`, check it, and generate the C file of the
/// descriptor and the header of the setting handles. Return the manifest,
/// the generated C, and the header.
pub fn generate(directory: &Path) -> Result<(Manifest, String, String)> {
    use grkit_core::packaging::Language;
    let path = directory.join("module.toml");
    let manifest: Manifest = toml::from_str(
        &fs::read_to_string(&path).with_context(|| format!("Read {}", path.display()))?,
    )
    .with_context(|| format!("Read {}", path.display()))?;
    let m = &manifest;
    ensure!(
        grkit_core::grm::valid_id(&m.id),
        "Invalid module ID {:?}",
        m.id
    );
    ensure!(
        m.order <= grkit_core::grm::MAX_ORDER,
        "order must be 0 to {}",
        grkit_core::grm::MAX_ORDER
    );
    // The C reference of a setting of a page or a layer (GRKIT_USE in
    // runtime/sdk/grkit.h): its own setting by key, or a used setting by its
    // full name. Also the number of values of an own setting.
    let setting_reference = |name: &str| -> Result<(String, Option<usize>)> {
        if let Some(index) = m.settings.iter().position(|s| s.key == name) {
            return Ok((index.to_string(), Some(m.settings[index].values as usize)));
        }
        let index = m
            .uses
            .iter()
            .position(|used| used.strip_prefix('?').unwrap_or(used) == name)
            .with_context(|| format!("module.toml has no setting {name} and does not use it"))?;
        Ok((format!("GRKIT_USE({index})"), None))
    };
    for (i, setting) in m.settings.iter().enumerate() {
        ensure!(
            valid_key(&setting.key),
            "Invalid setting key {:?}",
            setting.key
        );
        ensure!(
            (1..=grkit_core::runtime::MODULE_SETTING_VALUES as u32).contains(&setting.values),
            "Setting {} needs 1 to {} values",
            setting.key,
            grkit_core::runtime::MODULE_SETTING_VALUES
        );
        ensure!(
            m.settings[..i].iter().all(|s| s.key != setting.key),
            "Setting {} is twice in module.toml",
            setting.key
        );
    }
    for name in &m.uses {
        let full = name.strip_prefix('?').unwrap_or(name);
        let (module, key) = full
            .split_once('.')
            .context("A used setting is MODULE.KEY")?;
        ensure!(
            grkit_core::grm::valid_id(module) && valid_key(key),
            "Invalid used setting {name:?}"
        );
    }
    ensure!(
        m.tiles.len() <= grkit_core::runtime::MODULE_TILES_MAX,
        "More than {} tiles",
        grkit_core::runtime::MODULE_TILES_MAX
    );
    let mut tiles = Vec::new();
    for tile in &m.tiles {
        let path = directory.join(tile);
        let (width, height, rgba) = grkit_core::png::decode_rgba8(
            &fs::read(&path).with_context(|| format!("Read {}", path.display()))?,
        )
        .with_context(|| format!("Read {}", path.display()))?;
        ensure!(width == 40 && height == 40, "{tile} is not 40 x 40 pixels");
        tiles.extend(rgba);
    }

    let mut c = String::from(
        "/* Generated by grkit module build from module.toml. Do not edit. */\n#include \"grkit.h\"\n",
    );
    let mut header = format!(
        "/* Generated by grkit module build from module.toml. Do not edit. */\n#ifndef GRKIT_MODULE_{id}_H\n#define GRKIT_MODULE_{id}_H\n#include \"grkit.h\"\n",
        id = m.id
    );
    for (i, setting) in m.settings.iter().enumerate() {
        writeln!(header, "#define SETTING_{} GRKIT_SETTING({i})", setting.key)?;
    }
    for (i, name) in m.uses.iter().enumerate() {
        let full = name.strip_prefix('?').unwrap_or(name);
        writeln!(
            header,
            "#define USED_{} GRKIT_USED({i})",
            full.replace('.', "_")
        )?;
    }
    header.push_str("#endif\n");

    let mut registrations = Vec::new();
    let core_page = m
        .page
        .as_ref()
        .filter(|page| CORE_ROWS.contains(&page.setting.as_str()));
    if let Some(page) = core_page {
        // The core gives the rows: the page has only its title.
        let (setting, _) = setting_reference(&page.setting)?;
        ensure!(
            !page.off_row && page.settings.is_empty(),
            "A page of {} has no off_row and no settings",
            page.setting
        );
        let mut titles = Vec::new();
        for (l, language) in Language::ALL.into_iter().enumerate() {
            let path = directory.join(format!("text/{}.toml", language.code()));
            let text: PageText = toml::from_str(
                &fs::read_to_string(&path).with_context(|| format!("Read {}", path.display()))?,
            )
            .with_context(|| format!("Read {}", path.display()))?;
            ensure!(
                text.rows.is_empty() && text.summaries.is_none() && text.settings.is_empty(),
                "{}: the core gives the rows of {}, so the text has only the title",
                path.display(),
                page.setting
            );
            writeln!(c, "static const u16 title_{l}[] = {};", utf16(&text.title)?)?;
            titles.push(format!("title_{l}"));
        }
        writeln!(
            c,
            "static const u16 *const page_title[] = {{{}}};",
            titles.join(",")
        )?;
        writeln!(
            c,
            "static const GrkitPage page = {{sizeof(GrkitPage), GRKIT_PAGE_CHOICE, {setting}, GRKIT_CHOICE_CORE_ROWS{}, page_title, 0, 0, 0, 0, 0}};",
            if page.options { " | GRKIT_CHOICE_OPTIONS" } else { "" }
        )?;
        registrations.push("{GRKIT_POINT_MENU_PAGE, &page}".to_owned());
    }
    if let Some(page) = m.page.as_ref().filter(|_| core_page.is_none()) {
        // A used setting has one value for each row of the English text. The
        // installer checks that number.
        let (setting, own_values) = setting_reference(&page.setting)?;
        let english = directory.join(format!("text/{}.toml", Language::ALL[0].code()));
        let read_text = |path: &Path| -> Result<PageText> {
            toml::from_str(
                &fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?,
            )
            .with_context(|| format!("Read {}", path.display()))
        };
        let values = match own_values {
            Some(values) => values,
            None => read_text(&english)?.rows.len(),
        };
        ensure!((1..=64).contains(&values), "A choice page has 1 to 64 rows");
        ensure!(
            m.tiles.len() >= values,
            "A choice page needs a tile for each value"
        );
        ensure!(
            page.settings.is_empty()
                || page.settings.len() <= grkit_core::runtime::NATIVE_SETTING_ROWS,
            "A page has at most {} rows of a native settings page",
            grkit_core::runtime::NATIVE_SETTING_ROWS
        );
        // Row 0 is the page setting, also a used one such as CORE.ASPECT:
        // its values are the rows of the page. The other rows are settings
        // of the module. INSTALL writes the handle of each row.
        let mut setting_rows = Vec::new();
        for (i, key) in page.settings.iter().enumerate() {
            let (reference, count) = if i == 0 {
                ensure!(
                    key == &page.setting,
                    "The first native settings row is the page setting"
                );
                (setting.clone(), values)
            } else {
                let index = m
                    .settings
                    .iter()
                    .position(|s| &s.key == key)
                    .with_context(|| {
                        format!(
                            "A native settings row names {key}, which is not a setting of the module"
                        )
                    })?;
                (index.to_string(), m.settings[index].values as usize)
            };
            ensure!(
                count <= grkit_core::runtime::NATIVE_SETTING_VALUES,
                "A native settings row has at most {} values",
                grkit_core::runtime::NATIVE_SETTING_VALUES
            );
            setting_rows.push((key.clone(), reference, count));
        }
        let (mut titles, mut summaries, mut labels) = (Vec::new(), Vec::new(), Vec::new());
        let mut row_labels = vec![Vec::new(); setting_rows.len()];
        let mut row_values = vec![Vec::new(); setting_rows.len()];
        for (l, language) in Language::ALL.into_iter().enumerate() {
            let path = directory.join(format!("text/{}.toml", language.code()));
            let text = read_text(&path)?;
            ensure!(
                text.rows.len() == values,
                "{} has {} rows, and {} has {values} values",
                path.display(),
                text.rows.len(),
                page.setting
            );
            let summary_values = text.summaries.as_ref().unwrap_or(&text.rows);
            ensure!(
                summary_values.len() == values,
                "{} has {} summaries, and {} has {values} values",
                path.display(),
                summary_values.len(),
                page.setting
            );
            writeln!(c, "static const u16 title_{l}[] = {};", utf16(&text.title)?)?;
            titles.push(format!("title_{l}"));
            let separator = grkit_core::menu_text::separator(language);
            for (row, (label, value)) in text.rows.iter().zip(summary_values).enumerate() {
                writeln!(c, "static const u16 label_{row}_{l}[] = {};", utf16(label)?)?;
                writeln!(
                    c,
                    "static const u16 summary_{row}_{l}[] = {};",
                    utf16(&format!("{}{separator}{value}", text.title))?
                )?;
            }
            ensure!(
                text.settings.len() == setting_rows.len(),
                "{} names the text of {} native settings rows, and the page has {}",
                path.display(),
                text.settings.len(),
                setting_rows.len()
            );
            for (r, (key, _, values)) in setting_rows.iter().enumerate() {
                let row = text
                    .settings
                    .get(key)
                    .with_context(|| format!("{} has no [settings.{key}]", path.display()))?;
                writeln!(
                    c,
                    "static const u16 row_{r}_{l}[] = {};",
                    utf16(&row.label)?
                )?;
                row_labels[r].push(format!("row_{r}_{l}"));
                let texts = match (&row.values, r) {
                    (Some(texts), _) => texts,
                    (None, 0) => &text.rows,
                    (None, _) => bail!("{} gives no values of {key}", path.display()),
                };
                ensure!(
                    texts.len() == *values,
                    "{} has {} values of {key}, and {key} has {values}",
                    path.display(),
                    texts.len()
                );
                for (v, value) in texts.iter().enumerate() {
                    writeln!(c, "static const u16 row_{r}_{v}_{l}[] = {};", utf16(value)?)?;
                }
            }
        }
        for row in 0..values {
            for l in 0..Language::ALL.len() {
                labels.push(format!("label_{row}_{l}"));
                summaries.push(format!("summary_{row}_{l}"));
            }
        }
        writeln!(
            c,
            "static const u16 *const page_title[] = {{{}}};",
            titles.join(",")
        )?;
        writeln!(
            c,
            "static const u16 *const page_summaries[] = {{{}}};",
            summaries.join(",")
        )?;
        writeln!(
            c,
            "static const u16 *const page_labels[] = {{{}}};",
            labels.join(",")
        )?;
        for (r, (_, reference, values)) in setting_rows.iter().enumerate() {
            writeln!(
                c,
                "static const u16 *const row_{r}_label[] = {{{}}};",
                row_labels[r].join(",")
            )?;
            let texts: Vec<_> = (0..*values)
                .flat_map(|v| (0..Language::ALL.len()).map(move |l| format!("row_{r}_{v}_{l}")))
                .collect();
            writeln!(
                c,
                "static const u16 *const row_{r}_values[] = {{{}}};",
                texts.join(",")
            )?;
            row_values[r].push(format!(
                "{{sizeof(GrkitSettingRow), {reference}, row_{r}_label, row_{r}_values}}"
            ));
        }
        let settings_field = if setting_rows.is_empty() {
            "0, 0".to_owned()
        } else {
            writeln!(
                c,
                "static const GrkitSettingRow page_settings[] = {{{}}};",
                row_values.concat().join(", ")
            )?;
            format!("{}, page_settings", setting_rows.len())
        };
        ensure!(
            !page.options || page.off_row,
            "A page with options has an Off row, which stays on"
        );
        ensure!(
            !page.options || values <= 16,
            "A page with options has at most 16 values"
        );
        let flags = match (page.off_row, page.options) {
            (true, true) => "GRKIT_CHOICE_OFF_ROW | GRKIT_CHOICE_OPTIONS",
            (true, false) => "GRKIT_CHOICE_OFF_ROW",
            _ => "0",
        };
        writeln!(
            c,
            "static const GrkitPage page = {{sizeof(GrkitPage), GRKIT_PAGE_CHOICE, {setting}, {flags}, page_title, page_summaries, page_labels, {values}, {settings_field}}};"
        )?;
        registrations.push("{GRKIT_POINT_MENU_PAGE, &page}".to_owned());
    }
    for (i, layer) in m.layers.iter().enumerate() {
        let paint = c_name(&layer.paint)?;
        ensure!(layer.order <= 0xffff, "A layer order is at most 65535");
        let setting = match &layer.setting {
            Some(name) => setting_reference(name)?.0,
            None => "GRKIT_NO_SETTING".to_owned(),
        };
        ensure!(
            !(layer.follow_disp && layer.full_disp),
            "A layer has follow_disp or full_disp, not both"
        );
        let flags: Vec<_> = [
            (layer.follow_disp, "GRKIT_LAYER_FOLLOW_DISP"),
            (layer.full_disp, "GRKIT_LAYER_FULL_DISP"),
            (layer.live_only, "GRKIT_LAYER_LIVE_ONLY"),
        ]
        .into_iter()
        .filter_map(|(set, flag)| set.then_some(flag))
        .collect();
        let flags = if flags.is_empty() {
            "0".to_owned()
        } else {
            flags.join(" | ")
        };
        writeln!(c, "void {paint}(const Surface *s, u32 value);")?;
        writeln!(
            c,
            "static const GrkitLayer layer_{i} = {{sizeof(GrkitLayer), {flags}, {paint}, {setting}, {}}};",
            layer.order
        )?;
        registrations.push(format!("{{GRKIT_POINT_OVERLAY_LAYER, &layer_{i}}}"));
    }
    for (i, filter) in m.filters.iter().enumerate() {
        let apply = c_name(&filter.apply)?;
        let stage = match filter.stage.as_str() {
            "frame" => "GRKIT_STAGE_FRAME",
            "mark" => "GRKIT_STAGE_MARK",
            other => bail!("Unknown filter stage {other:?}: use frame or mark"),
        };
        let setting = setting_reference(&filter.setting)?.0;
        writeln!(c, "void {apply}(PictureSet *set, u32 value);")?;
        writeln!(
            c,
            "static const GrkitPlaneFilter filter_{i} = {{sizeof(GrkitPlaneFilter), {stage}, {setting}, {apply}}};"
        )?;
        registrations.push(format!("{{GRKIT_POINT_PICTURE_PLANE, &filter_{i}}}"));
    }
    if let Some(events) = &m.events {
        let handle = c_name(&events.handle)?;
        ensure!(!events.on.is_empty(), "[events] names no event");
        let mut bits = Vec::new();
        for event in &events.on {
            let bit = match event.as_str() {
                "system_ready" => "GRKIT_EVENT_SYSTEM_READY",
                "setting_changed" => "GRKIT_EVENT_SETTING_CHANGED",
                other => bail!("Unknown event {other:?}: use system_ready or setting_changed"),
            };
            ensure!(!bits.contains(&bit), "[events] names {event} twice");
            bits.push(bit);
        }
        writeln!(c, "void {handle}(u32 event, u32 a, u32 b);")?;
        writeln!(
            c,
            "static const GrkitSubscriber subscriber = {{sizeof(GrkitSubscriber), {}, {handle}}};",
            bits.join(" | ")
        )?;
        registrations.push("{GRKIT_POINT_EVENT, &subscriber}".to_owned());
    }
    ensure!(
        !registrations.is_empty(),
        "module.toml has no page, no layer, no filter, and no events"
    );
    writeln!(
        c,
        "static const GrkitRegistration registrations[] = {{{}}};",
        registrations.join(", ")
    )?;
    let settings = if m.settings.is_empty() {
        "0, 0".to_owned()
    } else {
        let entries: Vec<_> = m
            .settings
            .iter()
            .map(|s| {
                let flags = match (s.saved, s.public) {
                    (true, true) => "GRKIT_SETTING_SAVED | GRKIT_SETTING_PUBLIC",
                    (true, false) => "GRKIT_SETTING_SAVED",
                    (false, true) => "GRKIT_SETTING_PUBLIC",
                    (false, false) => "0",
                };
                format!(
                    "{{sizeof(GrkitSetting), \"{}\", {}, {flags}}}",
                    s.key, s.values
                )
            })
            .collect();
        writeln!(
            c,
            "static const GrkitSetting settings[] = {{{}}};",
            entries.join(", ")
        )?;
        format!("settings, {}", m.settings.len())
    };
    let uses = if m.uses.is_empty() {
        "0, 0".to_owned()
    } else {
        let names: Vec<_> = m.uses.iter().map(|name| format!("\"{name}\"")).collect();
        writeln!(
            c,
            "static const char *const uses[] = {{{}}};",
            names.join(", ")
        )?;
        format!("uses, {}", m.uses.len())
    };
    let tile_field = if tiles.is_empty() {
        "0, 0".to_owned()
    } else {
        c.push_str("static const u8 tiles[] __attribute__((aligned(4))) = {");
        for (i, byte) in tiles.iter().enumerate() {
            if i % 32 == 0 {
                c.push('\n');
            }
            write!(c, "{byte},")?;
        }
        c.push_str("\n};\n");
        format!("tiles, {}", m.tiles.len())
    };
    writeln!(
        c,
        "__attribute__((used)) const GrkitModule grkit_module = {{sizeof(GrkitModule), GRKIT_ABI, registrations, {}, {settings}, {uses}, {tile_field}}};",
        registrations.len()
    )?;
    Ok((manifest, c, header))
}

/// Build the module of the manifest in `directory` (`module.toml`).
/// `objects` receives the generated files and the objects.
pub fn build_manifest(
    directory: &Path,
    sdk: &Path,
    objects: &Path,
    tools: &Toolchain,
) -> Result<Module> {
    let (manifest, c, header) = generate(directory)?;
    let generated = objects.join("grkit_module.c");
    fs::write(&generated, c)?;
    fs::write(objects.join("grkit_module.h"), header)?;
    let mut sources = vec![generated];
    for source in &manifest.sources {
        sources.push(
            directory
                .join(source)
                .canonicalize()
                .with_context(|| format!("Read {source}"))?,
        );
    }
    let sources: Vec<&Path> = sources.iter().map(|p| p.as_path()).collect();
    let mut module = build_with(
        &manifest.id,
        manifest.version,
        &sources,
        &[sdk, objects],
        objects,
        tools,
    )?;
    module.order = manifest.order;
    Ok(module)
}

/// Run the host test of the module in `directory`: its `test.c`, with the
/// module sources, the generated descriptor, and the harness of the SDK
/// (`runtime/sdk/grkit_test.h`), built with the host compiler `cc`. A module
/// without sources needs no `test.c`: then only its manifest is checked.
/// Return whether a test ran.
pub fn test_manifest(directory: &Path, sdk: &Path, objects: &Path, cc: &Path) -> Result<bool> {
    let (manifest, c, header) = generate(directory)?;
    let test = directory.join("test.c");
    if manifest.sources.is_empty() && !test.is_file() {
        return Ok(false);
    }
    ensure!(test.is_file(), "{} has no test.c", directory.display());
    let generated = objects.join("grkit_module.c");
    fs::write(&generated, c)?;
    fs::write(objects.join("grkit_module.h"), header)?;
    let executable = objects.join("module-test");
    let mut arguments: Vec<String> = [
        "-std=c11",
        "-Wall",
        "-Werror",
        "-Wno-unused-function",
        "-include",
    ]
    .map(String::from)
    .to_vec();
    arguments.push(path(&sdk.join("grkit_test.h"))?.to_owned());
    for include in [sdk, objects] {
        arguments.push("-I".into());
        arguments.push(path(include)?.to_owned());
    }
    arguments.push(path(&generated)?.to_owned());
    for source in &manifest.sources {
        arguments.push(path(&directory.join(source))?.to_owned());
    }
    arguments.push(path(&test)?.to_owned());
    arguments.push(path(&sdk.join("grkit_test.c"))?.to_owned());
    arguments.push("-o".into());
    arguments.push(path(&executable)?.to_owned());
    run(cc, arguments.iter().map(String::as_str))?;
    let output = std::process::Command::new(&executable)
        .output()
        .with_context(|| format!("Run {}", executable.display()))?;
    ensure!(
        output.status.success(),
        "The test of {} failed\n{}{}",
        manifest.id,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(true)
}
