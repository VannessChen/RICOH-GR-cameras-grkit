//! Ports the firmware addresses of a target to another build of the firmware:
//! a firmware version whose code moved by more than the relocation radius of
//! the installer, or whose code changed (`docs/architecture.md`, Porting).
//!
//! The port takes each address that the target names: its hook and patch
//! sites (`components::Layout`), the `FIRMWARE()` addresses of its runtime
//! header, and the data addresses of `Target`. It finds each one in the other
//! build with the windows of the installer (`crate::relocation`), within a
//! larger radius, and gives data that code loads that code as a second
//! source. The family header has no addresses (a test checks it). An address whose code or data differs in the other build
//! has no port, and a person must port it.
//!
//! The port does not check values that are not addresses: object offsets, text
//! IDs, enum values, and the offsets that header expressions add to an
//! address. The draft header marks each expression with an offset.

use crate::{
    components::Layout,
    firmware::FirmwareImage,
    loader::firmware_symbol,
    relocation::{self, ImageLayout, Planner},
    runtime_source,
    target::{Target, RTOS_BASE},
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fmt::Write};

/// The default search range on each side of an address. GR IV 1.20 moved
/// code by up to 0x29000 and `.data` by up to 0x37000 from 1.11. A larger
/// range needs larger windows, which more often include changed code.
pub const RADIUS: u32 = 0x80000;
/// The largest radius: more than the RTOS image, and twice it fits in a word.
pub const MAX_RADIUS: u32 = 0x0400_0000;

/// Where an address of a target comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A hook or patch site of the layout. Its stock bytes change with it.
    Site,
    /// A `FIRMWARE()` address in the runtime header of the target.
    Header,
    /// A data address of `Target` or of its Image Control.
    Target,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Site => "site",
            Kind::Header => "header",
            Kind::Target => "target",
        }
    }
}

/// An address that a target names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub kind: Kind,
    /// `<table>:<symbol>` or `<table>:<index>` for a site, the definition name
    /// for a header address (with `#<n>` after the first one in a definition),
    /// or the field of `Target`.
    pub name: String,
    pub address: u32,
    /// The stock bytes at a site.
    pub stock: Option<String>,
    /// For a site: its symbol, if its table has one, and the Rust of the
    /// fields after the stock bytes in its table row (`components.rs`).
    pub symbol: Option<&'static str>,
    pub rest: String,
}

/// The sites of a layout, by table, in table order.
fn sites(layout: &Layout) -> Vec<Address> {
    let mut sites = Vec::new();
    let mut add = |table: &str,
                   symbol: Option<&'static str>,
                   index: usize,
                   address: u32,
                   stock: Option<&str>,
                   rest: String| {
        sites.push(Address {
            kind: Kind::Site,
            name: format!(
                "{table}:{}",
                symbol.map_or(index.to_string(), str::to_string)
            ),
            address,
            stock: stock.map(str::to_string),
            symbol,
            rest,
        });
    };
    for (index, &(address, symbol, flag)) in layout.processing_hooks.iter().enumerate() {
        add(
            "processing",
            Some(symbol),
            index,
            address,
            None,
            format!("{flag}"),
        );
    }
    for (index, &(address, stock)) in layout.contexts.iter().enumerate() {
        add("context", None, index, address, Some(stock), String::new());
    }
    for (index, &(address, symbol, stock)) in layout.menu_hooks.iter().enumerate() {
        add(
            "menu",
            Some(symbol),
            index,
            address,
            Some(stock),
            String::new(),
        );
    }
    for (table, hooks) in [
        ("viewfinder", layout.viewfinder_hooks),
        ("metadata", layout.metadata_hooks),
        ("parameter", layout.parameter_hooks),
        ("settings", layout.settings_hooks),
        ("framing", layout.framing_hooks),
        ("picture", layout.picture_hooks),
    ] {
        for (index, &(address, symbol, stock, flag)) in hooks.iter().enumerate() {
            add(
                table,
                Some(symbol),
                index,
                address,
                Some(stock),
                format!("{flag}"),
            );
        }
    }
    for (table, patches) in [
        ("instruction", layout.instruction_patches),
        ("framing_patch", layout.framing_patches),
    ] {
        for (index, &(address, stock, after)) in patches.iter().enumerate() {
            add(
                table,
                None,
                index,
                address,
                Some(stock),
                format!("{after:?}"),
            );
        }
    }
    // The startup literals stay at their address in each build.
    for (index, &(address, stock)) in layout.guards.iter().enumerate() {
        if address != relocation::STARTUP_LITERALS {
            add("guard", None, index, address, Some(stock), String::new());
        }
    }
    sites
}

/// Each definition of a runtime header, as (name, value), with continuation
/// lines joined.
fn definitions(header: &str) -> Vec<(String, String)> {
    header
        .replace("\\\n", " ")
        .lines()
        .filter_map(|line| {
            let definition = line.strip_prefix("#define ")?;
            let (name, value) = definition.split_once(' ').unwrap_or((definition, ""));
            Some((name.to_string(), value.trim().to_string()))
        })
        .collect()
}

/// The `FIRMWARE()` addresses of a definition value, in order.
fn firmware_addresses(value: &str) -> Vec<u32> {
    value
        .split("FIRMWARE(")
        .skip(1)
        .filter_map(|part| {
            let number = part.split(')').next()?.trim();
            u32::from_str_radix(number.strip_prefix("0x")?, 16).ok()
        })
        .collect()
}

/// The runtime header of a target, without the family header that it includes.
fn header(target: &Target) -> Result<&'static str> {
    let path = target.header();
    let bytes = runtime_source(&path).with_context(|| format!("No runtime/{path}"))?;
    Ok(std::str::from_utf8(bytes)?)
}

/// Every address that `target` names: its sites, its header addresses, then
/// its data addresses.
pub fn addresses(target: &Target) -> Result<Vec<Address>> {
    let layout = target
        .layout
        .with_context(|| format!("{} has no layout", target.name))?;
    let mut addresses = sites(layout);
    for (name, value) in definitions(header(target)?) {
        for (index, address) in firmware_addresses(&value).into_iter().enumerate() {
            addresses.push(Address {
                kind: Kind::Header,
                name: if index == 0 {
                    name.clone()
                } else {
                    format!("{name}#{index}")
                },
                address,
                stock: None,
                symbol: None,
                rest: String::new(),
            });
        }
    }
    let mut data = vec![
        ("array_table".to_string(), target.array_table),
        ("array_count".to_string(), target.array_count),
        ("heap_min".to_string(), target.heap_min),
        ("english_slot".to_string(), target.english_slot),
        ("pixel_base_slot".to_string(), target.pixel_base_slot),
    ];
    if let Some(native) = target.image_control {
        for (index, &table) in native.tables.iter().enumerate() {
            data.push((format!("image_control.tables[{index}]"), table));
        }
        data.push(("image_control.record".into(), native.record));
        data.push(("image_control.coefficients".into(), native.coefficients));
        for (index, &address) in native.record_tail.iter().enumerate() {
            data.push((format!("image_control.record_tail[{index}]"), address));
        }
        data.push(("image_control.base_gamma".into(), native.base_gamma));
        data.push((
            "image_control.settings_domain".into(),
            native.settings_domain,
        ));
    }
    for (name, address) in data {
        addresses.push(Address {
            kind: Kind::Target,
            name,
            address,
            stock: None,
            symbol: None,
            rest: String::new(),
        });
    }
    Ok(addresses)
}

/// An address of the base target in the other build.
#[derive(Debug, Clone)]
pub struct Ported {
    pub address: Address,
    /// The address in the other build, or why the port did not find it.
    pub port: std::result::Result<u32, String>,
    /// The number of sources that agree on the port (`relocation::Resolution`).
    pub sources: usize,
    /// Why a person must check a port that the sources agree on.
    pub review: Option<String>,
    /// For a site: the bytes at the port, as many as the stock bytes.
    pub stock: Option<String>,
}

#[derive(Debug)]
pub struct Port {
    pub base: &'static Target,
    pub radius: u32,
    /// The layout of the other build.
    pub layout: ImageLayout,
    pub addresses: Vec<Ported>,
    /// Whether the installer relocates the base target to the other build
    /// (`relocatable`).
    pub relocatable: bool,
}

/// Adds the sources of an address to the plan. A window of data can match a
/// moved copy of the same words, such as the start of a table that grew, so a
/// data address that code loads also gets that code as a second source; the
/// sources must agree. A site keeps the window of its function, like the
/// installer.
fn plan(planner: &mut Planner, address: &Address, planned: u32) -> Result<usize> {
    let first = installer_symbol(planner, address, planned);
    if address.kind == Kind::Site || planned == planner.target().heap_min {
        return first;
    }
    let second = planner.referenced(planned);
    first.or(second)
}

/// The sources of an address as the installer plans them
/// (`loader::relocation_tables`): data of `Target` in the image by its
/// content, the others by the region that holds them.
fn installer_symbol(planner: &mut Planner, address: &Address, planned: u32) -> Result<usize> {
    if address.kind == Kind::Target && planned < planner.target().data_source {
        planner.content(planned)
    } else {
        firmware_symbol(planner, planned)
    }
}

/// The address that the plan takes for `address`, and the offset to add. An
/// address in .data that no code loads directly keeps its offset from the
/// address that code loads (`Target::relocation_bases`).
fn planned(target: &Target, address: &Address) -> (u32, u32) {
    target
        .relocation_bases
        .iter()
        .find(|(field, _)| *field == address.address)
        .map_or((address.address, 0), |&(field, base)| (base, field - base))
}

/// Why a port with only weak sources needs a check.
const WEAK: &str = "only windows of mostly addresses agree: a table that grew or moved \
    matches at another start";

/// A source in a window of mostly address words. A table of pointers masks
/// to equal words, so its window can match where the table grew or moved,
/// and a literal in it can then hold another entry. The code windows of
/// GR IV 1.11 hold at most a fifth of address words.
fn weak(plan: &relocation::Plan, source: &relocation::Source) -> bool {
    use relocation::Source;
    let (Source::Window { window, .. }
    | Source::Literal { window, .. }
    | Source::Pair { window, .. }
    | Source::Branch { window, .. }) = *source
    else {
        return false;
    };
    let words = &plan.windows[window].words;
    let addresses = words
        .iter()
        .filter(|&&word| word == relocation::ADDRESS_MASK)
        .count();
    addresses * 2 > words.len()
}

/// Entry 11 of the first Image Control type table points to the Standard
/// record (`FirmwareImage::stock_record`). A ported table start without it
/// needs a check.
fn check_image_control(addresses: &mut [Ported], other: &[u8], layout: &ImageLayout) {
    let port = |addresses: &[Ported], name: &str| {
        addresses
            .iter()
            .find(|ported| ported.address.kind == Kind::Target && ported.address.name == name)
            .and_then(|ported| ported.port.clone().ok())
    };
    let (Some(table), Some(record)) = (
        port(addresses, "image_control.tables[0]"),
        port(addresses, "image_control.record"),
    ) else {
        return;
    };
    let entry = table + 11 * 4;
    let holder = entry
        .checked_sub(relocation::DATA_START)
        .map(|offset| layout.data_source + offset - RTOS_BASE);
    let value = holder.and_then(|at| {
        other
            .get(at as usize..at as usize + 4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
    });
    if value != Some(record) {
        if let Some(ported) = addresses.iter_mut().find(|ported| {
            ported.address.kind == Kind::Target && ported.address.name == "image_control.tables[0]"
        }) {
            ported.review = Some(format!("entry 11 is not the Standard record {record:#x}"));
        }
    }
}

/// Ports each address of `base.target` to `other`, the RTOS image of another
/// build, loaded at `RTOS_BASE`.
pub fn port(base: &FirmwareImage, other: &[u8], radius: u32) -> Result<Port> {
    ensure!(
        radius.is_multiple_of(4) && (relocation::RADIUS..=MAX_RADIUS).contains(&radius),
        "The radius must be a multiple of 4 from {:#x} to {MAX_RADIUS:#x}",
        relocation::RADIUS
    );
    let target = base.target;
    let addresses = addresses(target)?;
    let mut planner = Planner::with_radius(base, radius)?;
    let mut symbols = Vec::new();
    for address in &addresses {
        let (planned, offset) = planned(target, address);
        symbols.push(
            plan(&mut planner, address, planned)
                .map(|symbol| (symbol, offset))
                .map_err(|error| format!("no window in {}: {error}", target.firmware)),
        );
    }
    let plan = planner.finish();
    let resolution = relocation::resolve(&plan, other, radius)?;
    let read = |address: u32, size: usize| -> Option<String> {
        let start = address.checked_sub(RTOS_BASE)? as usize;
        Some(hex::encode(other.get(start..start + size)?))
    };
    let addresses = addresses
        .into_iter()
        .zip(symbols)
        .map(|(address, symbol)| {
            let resolved = symbol.and_then(|(symbol, offset)| {
                resolution.symbols[symbol]
                    .clone()
                    .map(|(value, sources)| (value + offset, sources))
            });
            let review = match &resolved {
                Ok((_, sources)) if sources.iter().all(|source| weak(&plan, source)) => {
                    Some(WEAK.to_string())
                }
                _ => None,
            };
            let sources = resolved.as_ref().map_or(0, |(_, sources)| sources.len());
            let port = resolved.map(|(value, _)| value);
            let stock = match (&port, &address.stock) {
                (Ok(at), Some(stock)) => read(*at, stock.len() / 2),
                _ => None,
            };
            Ported {
                address,
                port,
                sources,
                review,
                stock,
            }
        })
        .collect();
    let mut addresses: Vec<Ported> = addresses;
    check_image_control(&mut addresses, other, &resolution.layout);
    let relocatable = relocatable(base, other, &addresses)?;
    Ok(Port {
        base: target,
        radius,
        layout: resolution.layout,
        addresses,
        relocatable,
    })
}

/// Whether the installer relocates the base target to the other build: each
/// address has a port without a review, and the plan of the installer, within
/// `relocation::RADIUS`, resolves each source of each address to that port,
/// with room for the private tail. The plan of a build has more symbols, such
/// as branch targets, so a test of the build on the other firmware must still
/// confirm it.
fn relocatable(base: &FirmwareImage, other: &[u8], addresses: &[Ported]) -> Result<bool> {
    if addresses
        .iter()
        .any(|ported| ported.port.is_err() || ported.review.is_some())
    {
        return Ok(false);
    }
    let target = base.target;
    let mut planner = Planner::new(base)?;
    let mut symbols = Vec::new();
    for ported in addresses {
        let (planned, offset) = planned(target, &ported.address);
        match installer_symbol(&mut planner, &ported.address, planned) {
            Ok(symbol) => symbols.push((symbol, offset)),
            Err(_) => return Ok(false),
        }
    }
    let resolution = relocation::resolve(&planner.finish(), other, relocation::RADIUS)?;
    let (tail, end) = target.layout.context("A port target has a layout")?.tail;
    Ok(resolution.complete
        && relocation::DATA_START - resolution.layout.tail >= end - tail
        && symbols
            .iter()
            .zip(addresses)
            .all(|(&(symbol, offset), ported)| {
                resolution.symbols[symbol]
                    .as_ref()
                    .is_ok_and(|&(value, _)| Ok(value + offset) == ported.port)
            }))
}

/// Compares a port with the addresses of an existing target of the other
/// build: the same names must have the ported addresses.
pub fn check(port: &Port, target: &Target) -> Result<Value> {
    let expected: BTreeMap<(Kind, String), u32> = addresses(target)?
        .into_iter()
        .map(|address| ((address.kind, address.name), address.address))
        .collect();
    let mut equal = 0;
    let mut differ = Vec::new();
    let mut unported = Vec::new();
    let mut review = Vec::new();
    for ported in &port.addresses {
        let key = (ported.address.kind, ported.address.name.clone());
        let Some(&expected) = expected.get(&key) else {
            continue;
        };
        let name = format!("{}:{}", key.0.name(), key.1);
        let entry = |address: u32| {
            json!({"name": name, "port": format!("{address:#x}"),
                "target": format!("{expected:#x}"), "review": ported.review})
        };
        match &ported.port {
            Ok(address) if ported.review.is_some() => review.push(entry(*address)),
            Ok(address) if *address == expected => equal += 1,
            Ok(address) => differ.push(entry(*address)),
            Err(_) => unported.push(name),
        }
    }
    // Ports that need a check do not count as equal or different.
    Ok(json!({
        "target": target.name,
        "equal": equal,
        "differ": differ,
        "review": review,
        "unported": unported,
    }))
}

impl Port {
    /// The addresses that have no port.
    pub fn unported(&self) -> impl Iterator<Item = &Ported> {
        self.addresses.iter().filter(|ported| ported.port.is_err())
    }

    /// The largest distance that a ported address moved.
    pub fn largest_move(&self) -> u32 {
        self.addresses
            .iter()
            .filter_map(|ported| Some(ported.port.as_ref().ok()?.abs_diff(ported.address.address)))
            .max()
            .unwrap_or(0)
    }

    /// Each address with its port, for a report file.
    pub fn report(&self) -> Value {
        let addresses: Vec<Value> = self
            .addresses
            .iter()
            .map(|ported| {
                json!({
                    "kind": ported.address.kind.name(),
                    "name": ported.address.name,
                    "base": format!("{:#x}", ported.address.address),
                    "port": ported.port.as_ref().map(|at| format!("{at:#x}")).ok(),
                    "error": ported.port.as_ref().err(),
                    "sources": ported.sources,
                    "review": ported.review,
                    "stock": ported.address.stock,
                    "port_stock": ported.stock,
                })
            })
            .collect();
        json!({
            "base": self.base.name,
            "radius": format!("{:#x}", self.radius),
            "data_source": format!("{:#x}", self.layout.data_source),
            "data_size": format!("{:#x}", self.layout.data_size),
            "heap_min": format!("{:#x}", self.layout.heap_min),
            "tail": format!("{:#x}", self.layout.tail),
            "addresses": addresses,
        })
    }

    fn find(&self, kind: Kind, name: &str) -> Option<&Ported> {
        self.addresses
            .iter()
            .find(|ported| ported.address.kind == kind && ported.address.name == name)
    }

    /// The runtime header of the base target with each `FIRMWARE()` address
    /// ported. A comment marks each address without a port and each
    /// expression that adds an offset, which the port does not check.
    pub fn draft_header(&self) -> Result<String> {
        let mut out = String::from(
            "/* Draft from `grkit port`. Comments still describe the base build, and\n   \
             values that are not addresses are those of the base build. */\n",
        );
        let source = header(self.base)?.replace("\\\n", "\u{0}");
        for line in source.lines() {
            let line = line.replace('\u{0}', "\\\n");
            let Some(definition) = line.strip_prefix("#define ") else {
                out.push_str(&line);
                out.push('\n');
                continue;
            };
            let (name, value) = definition.split_once(' ').unwrap_or((definition, ""));
            let mut ported = line.clone();
            let mut notes = Vec::new();
            for (index, address) in firmware_addresses(value).into_iter().enumerate() {
                let key = if index == 0 {
                    name.to_string()
                } else {
                    format!("{name}#{index}")
                };
                let old = format!("FIRMWARE({address:#x})");
                let found = self.find(Kind::Header, &key);
                match found.map(|p| &p.port) {
                    Some(Ok(at)) => {
                        ported = ported.replacen(&old, &format!("FIRMWARE({at:#x})"), 1)
                    }
                    _ => notes.push(format!("no port for {address:#x}")),
                }
                if let Some(reason) = found.and_then(|p| p.review.as_ref()) {
                    notes.push(format!("check {address:#x}: {reason}"));
                }
            }
            if value.contains("FIRMWARE(") && value.contains(") +") {
                notes.push("check the offset".into());
            }
            out.push_str(&ported);
            if !notes.is_empty() {
                write!(out, " /* PORT: {} */", notes.join("; "))?;
            }
            out.push('\n');
        }
        Ok(out)
    }

    /// The site tables of the base layout at the ported sites, with the stock
    /// bytes of the other build, as Rust items. A comment marks each site
    /// without a port; it keeps the base address and stock bytes.
    pub fn draft_layout(&self) -> Result<String> {
        let mut out = format!(
            "// Draft from `grkit port`: the sites of {} in the other build.\n",
            self.base.name
        );
        let mut table = "";
        for ported in self
            .addresses
            .iter()
            .filter(|ported| ported.address.kind == Kind::Site)
        {
            let site = &ported.address;
            let name = site.name.split_once(':').map_or("", |(name, _)| name);
            if name != table {
                if !table.is_empty() {
                    out.push_str("];\n");
                }
                writeln!(out, "const {}: &[_] = &[", name.to_uppercase())?;
                table = name;
            }
            let (address, stock, note) = match (&ported.port, &ported.stock) {
                (Ok(at), stock) => (
                    *at,
                    stock.clone(),
                    ported
                        .review
                        .as_ref()
                        .map_or(String::new(), |reason| format!(" // PORT: check: {reason}")),
                ),
                (Err(error), _) => (
                    site.address,
                    site.stock.clone(),
                    format!(" // PORT: {error}"),
                ),
            };
            let fields: Vec<String> = [Some(format!("{address:#x}"))]
                .into_iter()
                .chain([
                    site.symbol.map(|symbol| format!("{symbol:?}")),
                    stock.map(|stock| format!("{stock:?}")),
                    (!site.rest.is_empty()).then(|| site.rest.clone()),
                ])
                .flatten()
                .collect();
            writeln!(out, "    ({}),{note}", fields.join(", "))?;
        }
        if !table.is_empty() {
            out.push_str("];\n");
        }
        let mut fields = String::new();
        for ported in self
            .addresses
            .iter()
            .filter(|ported| ported.address.kind == Kind::Target)
        {
            match &ported.port {
                Ok(at) if ported.review.is_some() => writeln!(
                    fields,
                    "//   {}: {at:#x} PORT: check: {}",
                    ported.address.name,
                    ported.review.as_deref().unwrap_or_default()
                )?,
                Ok(at) => writeln!(fields, "//   {}: {at:#x}", ported.address.name)?,
                Err(error) => writeln!(
                    fields,
                    "//   {}: {:#x} PORT: {error}",
                    ported.address.name, ported.address.address
                )?,
            }
        }
        write!(
            out,
            "// Target: data_source {:#x}, data_size {:#x}, heap_min {:#x}, tail {:#x}\n{fields}",
            self.layout.data_source, self.layout.data_size, self.layout.heap_min, self.layout.tail
        )?;
        Ok(out)
    }
}
