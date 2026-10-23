use super::*;

/// The version of headers that tests pin, so a new GRKit version keeps the pins.
const TEST_VERSION: &str = "0.0.0";

/// GR III without relocation. The synthetic image is mostly zero words, so
/// it has no unique relocation windows.
static FIXED_LAYOUT: components::Layout = components::Layout {
    relocatable: false,
    ..components::GR3_LAYOUT
};
static FIXED_GR3: target::Target = target::Target {
    layout: Some(&FIXED_LAYOUT),
    ..target::GR3_210
};

fn synthetic_firmware() -> firmware::FirmwareImage {
    let gr3 = &FIXED_GR3;
    let mut image = firmware::FirmwareImage {
        target: gr3,
        body: vec![0; (gr3.data_source - target::RTOS_BASE + gr3.data_size) as usize],
        sha256: "synthetic".into(),
        decoded_sha256: "synthetic".into(),
    };
    let mut write = |address: u32, bytes: &[u8]| {
        let address = if (gr3.data_start..gr3.data_start + gr3.data_size).contains(&address) {
            address - gr3.data_start + gr3.data_source
        } else {
            address
        };
        let offset = (address - target::RTOS_BASE) as usize;
        image.body[offset..offset + bytes.len()].copy_from_slice(bytes);
    };
    for &(address, bytes) in components::CONTEXTS {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, _, bytes) in components::MENU_HOOKS {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, _, bytes, _) in components::FRAMING_HOOKS
        .iter()
        .chain(components::VIEWFINDER_HOOKS.iter())
        .chain(components::METADATA_HOOKS.iter())
        .chain(components::PICTURE_HOOKS.iter())
        .chain(components::PARAMETER_HOOKS.iter())
        .chain(components::SETTINGS_HOOKS.iter())
    {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, bytes, _) in components::INSTRUCTION_PATCHES
        .iter()
        .chain(components::FRAMING_PATCHES.iter())
    {
        write(address, &hex::decode(bytes).unwrap());
    }
    for &(address, bytes) in components::GR3_LAYOUT.guards {
        write(address, &hex::decode(bytes).unwrap());
    }
    for address in target::GR3_IMAGE_CONTROL.tables {
        write(address, &(0..104u8).collect::<Vec<_>>());
    }
    write(
        target::GR3_IMAGE_CONTROL.tables[0] + 11 * 4,
        &0x5501900cu32.to_le_bytes(),
    );
    write(0x5501900c, &(0..84u8).collect::<Vec<_>>());
    write(0x55019010, &0x53dc2754u32.to_le_bytes());
    // The English entry of the language table, as GR III 2.10 holds it.
    write(gr3.english_slot, &0x55004490u32.to_le_bytes());
    image
}

#[test]
fn decoder_rejects_truncation_and_supports_overlap() {
    for tail in [
        vec![],
        vec![0x80, 0x10],
        vec![0, 4, 0x80, 0, 8, 1],
        vec![0, 1, 0],
    ] {
        let mut blob = vec![0; 128];
        blob.extend(tail);
        assert!(firmware::decode(&blob).is_err());
    }
    let mut blob = vec![0; 128];
    blob.extend_from_slice(b"\x80\x03abc\x00\x00");
    assert_eq!(firmware::decode(&blob).unwrap(), b"abc");
    let mut overlap = vec![0; 128];
    overlap.extend_from_slice(b"\x00\x05\x40\x00a\x03\x01\x00\x00");
    assert_eq!(firmware::decode(&overlap).unwrap(), b"aaaaaaa");
}

#[test]
fn firmware_identity_rejected_before_decode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("firmware.bin");
    fs::write(&path, b"wrong firmware").unwrap();
    assert!(verify_firmware(&path, &target::GR3_210)
        .unwrap_err()
        .to_string()
        .contains("SHA-256"));
}

#[test]
fn firmware_address_mapping_and_record_contract() {
    let mut body = synthetic_firmware();
    assert_eq!(body.stock_record().unwrap().0, 0x5501900c);
    assert_eq!(
        body.read(target::GR3_210.data_start, 8).unwrap(),
        body.read(target::GR3_210.data_source, 8).unwrap()
    );
    assert!(body.read(0, 4).is_err());
    assert!(body.read(target::RTOS_BASE, usize::MAX).is_err());
    let offset = (target::GR3_210.data_source - target::RTOS_BASE + 0x19010) as usize;
    body.body[offset] ^= 1;
    assert!(body.stock_record().is_err());
}

#[test]
fn patch_and_branch_validation() {
    assert_eq!(arm::branch(0x1000, 0x1008, 14).unwrap(), [0, 0, 0, 0xea]);
    for target in [0x1009, 0x3000000] {
        assert!(arm::branch(0x1000, target, 14).is_err());
    }
    assert!(arm::elf_text(&[0; 64]).is_err());
    let patch = Patch {
        address: 0x1000,
        points: 0,
        before: vec![0; 4],
        after: target::HEADER_REFERENCE.to_le_bytes().to_vec(),
        site: None,
    };
    let mut plan = PatchPlan {
        patches: vec![patch.clone()],
        relocations: vec![(0, 0)],
        references: Vec::new(),
    };
    validate_plan(&plan).unwrap();
    for relocations in [
        vec![(0, 4)],
        vec![(0, 1)],
        vec![(2, 0)],
        vec![(0, 0), (0, 0)],
        vec![(0, usize::MAX - 3)],
    ] {
        plan.relocations = relocations;
        assert!(validate_plan(&plan).is_err());
    }
    plan.relocations.clear();
    plan.patches.push(patch);
    assert!(validate_plan(&plan)
        .unwrap_err()
        .to_string()
        .contains("Overlapping"));
}

#[test]
fn elf_parser_rejects_unresolved_text_and_invalid_section_ranges() -> Result<()> {
    let names = b"\0.text\0.shstrtab\0.rel.text\0";
    let names_offset = 52 + 4 * 40;
    let text_offset = names_offset + names.len();
    let mut blob = vec![0u8; text_offset + 4];
    blob[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    blob[18..20].copy_from_slice(&40u16.to_le_bytes());
    blob[32..36].copy_from_slice(&52u32.to_le_bytes());
    blob[46..48].copy_from_slice(&40u16.to_le_bytes());
    blob[48..50].copy_from_slice(&4u16.to_le_bytes());
    blob[50..52].copy_from_slice(&2u16.to_le_bytes());
    let section = |blob: &mut Vec<u8>, index: usize, fields: [u32; 10]| {
        for (field, value) in fields.into_iter().enumerate() {
            let start = 52 + index * 40 + field * 4;
            blob[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
    };
    section(
        &mut blob,
        1,
        [1, 1, 0, 0, text_offset as u32, 4, 0, 0, 4, 0],
    );
    section(
        &mut blob,
        2,
        [
            7,
            3,
            0,
            0,
            names_offset as u32,
            names.len() as u32,
            0,
            0,
            1,
            0,
        ],
    );
    blob[names_offset..text_offset].copy_from_slice(names);
    blob[text_offset..].copy_from_slice(&[0, 0, 0xa0, 0xe1]);
    assert_eq!(arm::elf_text(&blob)?.0, [0, 0, 0xa0, 0xe1]);
    for end in 0..blob.len() {
        assert!(arm::elf_text(&blob[..end]).is_err());
    }
    section(
        &mut blob,
        3,
        [17, 9, 0, 0, text_offset as u32, 8, 0, 1, 4, 8],
    );
    assert!(arm::elf_text(&blob)
        .unwrap_err()
        .to_string()
        .contains("Unresolved"));
    section(
        &mut blob,
        3,
        [17, 2, 0, 0, text_offset as u32, 4, 2, 0, 4, 0],
    );
    assert!(arm::elf_text(&blob)
        .unwrap_err()
        .to_string()
        .contains("symbol entry size"));
    // An allocated, writable section (as .data or .bss would be).
    section(
        &mut blob,
        3,
        [17, 1, 3, 0, text_offset as u32, 4, 0, 0, 4, 0],
    );
    assert!(arm::elf_text(&blob)
        .unwrap_err()
        .to_string()
        .contains("not part of the private code"));
    Ok(())
}

#[test]
fn embedded_inputs_and_missing_tool() {
    let sources = source_hashes();
    assert!(sources.contains_key("runtime/installer.c"));
    assert!(sources.contains_key("crates/grkit-build/src/firmware.rs"));
    assert!(
        Toolchain::discover("/nonexistent/grkit-clang", "/nonexistent/grkit-lld")
            .unwrap_err()
            .to_string()
            .contains("Required tool not found")
    );
}

fn tools() -> Result<Toolchain> {
    Toolchain::discover(
        &std::env::var("GRKIT_CLANG").unwrap_or_else(|_| "clang".into()),
        &std::env::var("GRKIT_LLD").unwrap_or_else(|_| "ld.lld".into()),
    )
}

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_arm_build_is_reproducible() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let body = synthetic_firmware();
    let first = build_image(&body, &directory.path().join("first"), &tools)?;
    let second = build_image(&body, &directory.path().join("second"), &tools)?;
    assert_eq!(first.files, second.files);
    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.manifest["startup_activation"], "Off");
    assert_eq!(first.manifest["profile"], "production");
    assert_eq!(first.manifest["version"], env!("CARGO_PKG_VERSION"));
    assert!(first.manifest["source_revision"].is_null());
    assert!(first.manifest["source_dirty"].is_null());
    assert!(!serde_json::to_string(&first.manifest)?.contains(directory.path().to_str().unwrap()));
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(first.output.join("build.json"))?)?;
    assert_eq!(saved, first.manifest);
    let generated = fs::read(first.output.join("objects/module_payload.h"))?;
    assert_eq!(
        first.manifest["generated_header_sha256"],
        sha256(&generated)
    );
    let installer = &first.files["SCRIPT/PLUGINS/INSTALL.BIN"];
    assert_eq!(&installer[..6], b"\x7fELF\x01\x01");
    assert_eq!(&installer[16..20], &[2, 0, 40, 0]);
    for item in first.manifest["files"].as_array().unwrap() {
        let bytes = &first.files[item["path"].as_str().unwrap()];
        assert_eq!(item["size"], bytes.len());
        assert_eq!(item["sha256"], sha256(bytes));
    }
    assert!(build_image(&body, &first.output, &tools).is_err());
    for (path, bytes) in first.files {
        assert_eq!(fs::read(first.output.join("sd").join(path))?, bytes);
    }
    if let Ok(destination) = std::env::var("GRKIT_SYNTHETIC_OUTPUT") {
        let destination = Path::new(&destination);
        build_image(&body, destination, &tools)?;
        fs::write(destination.join("synthetic-body.bin"), &body.body)?;
    }
    Ok(())
}

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn synthetic_arm_hooks_match_fixed_target_contract() -> Result<()> {
    let tools = tools()?;
    let directory = tempfile::tempdir()?;
    let runtime = directory.path().join("runtime");
    materialize_runtime(&runtime)?;
    let body = synthetic_firmware();
    fs::write(runtime.join("hooks.h"), components::hooks_header(&body)?)?;
    let layout = &FIXED_LAYOUT;
    let processing_region = layout.processing.unwrap();
    let framing_region = layout.framing.unwrap();
    let processing = components::processing(
        directory.path(),
        &runtime,
        &body,
        &tools,
        layout,
        processing_region,
    )?;
    let menu = components::menu(directory.path(), &runtime, &body, &tools, layout)?;
    let framing = components::framing(
        directory.path(),
        &runtime,
        &body,
        &tools,
        layout,
        framing_region,
    )?;

    // Reject changes to the widget's input and output context, not just its BL.
    for address in 0x531710bc..0x531710d8 {
        let mut changed = synthetic_firmware();
        changed.body[(address - target::RTOS_BASE) as usize] ^= 1;
        let error = components::processing(
            directory.path(),
            &runtime,
            &changed,
            &tools,
            layout,
            processing_region,
        )
        .err()
        .expect("Modified viewfinder instructions must be rejected");
        assert!(error
            .to_string()
            .contains("instructions changed at 0x531710bc"));
    }

    // Independent target contract: do not derive these expectations from component tables.
    let processing_hooks = [
        (0x536dc124, "module_parameters", "abfdffeb", true),
        (0x536dc28c, "module_processing", "940d08e3", false),
        (0x536dc68c, "module_packet", "903084e5", false),
        (0x536e8694, "module_gamma_flag", "9b30d4e5", false),
        (0x536e87c4, "module_gamma_curves", "891900eb", true),
    ];
    let menu_hooks = [
        (0x531a48f0, "menu_native_touch_0", "f431d0e5", false),
        (0x531a4904, "menu_native_touch_1", "f431d0e5", false),
        (0x531a4918, "menu_native_touch_2", "f431d0e5", false),
        (0x531a492c, "menu_native_touch_3", "f431d0e5", false),
        (0x531a4940, "menu_native_touch_4", "f431d0e5", false),
        (0x5319ff24, "menu_destroy", "0dc0a0e1", false),
        (0x531a4114, "menu_refresh", "0dc0a0e1", false),
        (0x531a4db8, "menu_event", "b030d1e1", false),
        (0x531a3e88, "menu_exit", "7022d0e5", false),
        (0x531a4cfc, "menu_changed", "0dc0a0e1", false),
        (0x533966fc, "menu_text_measure", "077193e7", false),
        (0x53397108, "menu_text_draw", "073193e7", false),
        (0x53209fb8, "menu_icon_width", "9e3200e3", false),
        (0x53209fec, "menu_icon_height", "9e3200e3", false),
        (0x5320a7cc, "menu_icon_guard", "9e1200e3", false),
        (0x5320a810, "menu_icon_descriptor", "033192e7", false),
        (0x531a0ba0, "menu_native_adj_apply", "0dc0a0e1", false),
        (0x531ab998, "menu_native_dialog_apply", "0dc0a0e1", false),
        (0x531ac270, "menu_native_page_event", "0dc0a0e1", false),
        (0x531ab1b0, "menu_native_page_exit", "0dc0a0e1", false),
        (0x531710c4, "menu_viewfinder_icon", "97e204eb", true),
        (0x53179b7c, "menu_playback_icon", "daf404eb", true),
        (0x532b7cfc, "menu_playback_detail", "eef1ffeb", true),
        // The Fn parameter page: start, kinds, snapshots, event, then the
        // nine getters, setters, and row texts.
        (0x531b1020, "menu_parameter_start", "0dc0a0e1", false),
        (0x531b1094, "menu_parameter_kinds", "d2eeffeb", true),
        (0x531b1060, "menu_parameter_snapshot_0", "c54804eb", true),
        (0x531b10c8, "menu_parameter_snapshot_1", "ab4804eb", true),
        (0x531b0eac, "menu_parameter_event", "b030d1e1", false),
        (0x532c283c, "menu_parameter_get_0", "0dc0a0e1", false),
        (0x532c285c, "menu_parameter_get_1", "0dc0a0e1", false),
        (0x532c287c, "menu_parameter_get_2", "0dc0a0e1", false),
        (0x532c289c, "menu_parameter_get_3", "0dc0a0e1", false),
        (0x532c28bc, "menu_parameter_get_4", "0dc0a0e1", false),
        (0x532c28dc, "menu_parameter_get_5", "0dc0a0e1", false),
        (0x532c28fc, "menu_parameter_get_6", "0dc0a0e1", false),
        (0x532c2a1c, "menu_parameter_get_7", "0dc0a0e1", false),
        (0x532c2a3c, "menu_parameter_get_8", "0dc0a0e1", false),
        (0x532bd784, "menu_parameter_set_0", "0dc0a0e1", false),
        (0x532bdcd8, "menu_parameter_set_1", "0dc0a0e1", false),
        (0x532be22c, "menu_parameter_set_2", "0dc0a0e1", false),
        (0x532be8a0, "menu_parameter_set_3", "0dc0a0e1", false),
        (0x532bef14, "menu_parameter_set_4", "0dc0a0e1", false),
        (0x532bf588, "menu_parameter_set_5", "0dc0a0e1", false),
        (0x532bfbfc, "menu_parameter_set_6", "0dc0a0e1", false),
        (0x532c0da8, "menu_parameter_set_7", "0dc0a0e1", false),
        (0x532c141c, "menu_parameter_set_8", "0dc0a0e1", false),
        (0x532abfc4, "menu_parameter_row_0", "d8770aeb", true),
        (0x532ac0e0, "menu_parameter_row_1", "a2770aeb", true),
        (0x532ac1fc, "menu_parameter_row_2", "6c770aeb", true),
        (0x532ac318, "menu_parameter_row_3", "36770aeb", true),
        (0x532ac434, "menu_parameter_row_4", "00770aeb", true),
        (0x532ac550, "menu_parameter_row_5", "ca760aeb", true),
        (0x532ac66c, "menu_parameter_row_6", "94760aeb", true),
        (0x532acc88, "menu_parameter_row_7", "95750aeb", true),
        (0x532acda4, "menu_parameter_row_8", "5f750aeb", true),
        // The settings page: ChangeController, the five kind table bases, then
        // the count, text, apply, and current methods of the two value pages.
        (0x5316c9d8, "menu_settings_change", "0dc0a0e1", false),
        (0x531c5d20, "menu_settings_kinds_r3_0", "b33345e3", true),
        (0x531c5d88, "menu_settings_kinds_r2", "b32345e3", true),
        (0x531c60f4, "menu_settings_kinds_r8", "b38345e3", true),
        (0x531c631c, "menu_settings_kinds_r5", "b35345e3", true),
        (0x531c63f0, "menu_settings_kinds_r3_1", "b33345e3", true),
        (0x531b3c84, "menu_settings_count_0", "0600a0e3", false),
        (0x531b5b34, "menu_settings_draw_0", "0dc0a0e1", false),
        (0x531b7514, "menu_settings_apply_0", "0dc0a0e1", false),
        (0x531b8ce4, "menu_settings_current_0", "0dc0a0e1", false),
        (0x531b3c8c, "menu_settings_count_1", "0200a0e3", false),
        (0x531b5b74, "menu_settings_draw_1", "0dc0a0e1", false),
        (0x531b7544, "menu_settings_apply_1", "0dc0a0e1", false),
        (0x531b8d18, "menu_settings_current_1", "0dc0a0e1", false),
    ];
    let menu_instructions = [(0x535c251c, "0020a083", "01288083")];
    let framing_hooks = [
        (0x5317681c, "framing_update", "0dc0a0e1", false),
        (0x53176de4, "framing_exit", "0dc0a0e1", false),
        (0x53170b50, "framing_destroy", "0dc0a0e1", false),
        (0x53391b14, "framing_canvas", "0dc0a0e1", false),
        (0x53391be0, "framing_children", "500800eb", true),
        (0x535423f4, "framing_capture_request_hook", "0010d4e5", true),
        (0x536a2e48, "framing_height_0", "d33ea0e3", true),
        (0x536a2e34, "framing_height_1", "a93ea0e3", true),
        (0x536a2e20, "framing_height_2", "763ea0e3", true),
        (0x536a2e08, "framing_height_3", "383400e3", true),
        (0x536a2f98, "framing_screennail_inset", "2610a0e3", true),
        (0x536a3050, "framing_thumbnail_inset", "1010a0e3", true),
        (0x532b39b8, "framing_classify", "0dfcffeb", true),
        (0x532b3cac, "framing_image_aspect", "50fbffea", false),
        (0x536de1a8, "module_gamma_dirty", "0e30d4e5", false),
        (0x536e4774, "picture_post", "e9ffffeb", true),
        (0x535047d4, "picture_encode_entry", "0dc0a0e1", false),
        (0x53608080, "picture_capture_tone", "e1fdfeeb", true),
        (0x536078f4, "picture_capture_parameters", "3302ffeb", true),
        (0x535031b0, "grain_particle", "9537d5e5", true),
        (0x5351361c, "grain_gate", "e7feffeb", true),
    ];
    let framing_instructions = [
        (0x53d553c8, "02010300", "00000300"),
        (0x534371d8, "1500008a", "1500001a"),
        (0x536a3228, "f83b0ae3", "080000ea"),
        (0x535ee8b4, "0c00000a", "0300000a"),
        (0x535ee8c0, "000052e3", "020052e1"),
    ];
    for (component, base, end, expected, instructions) in [
        (
            &processing,
            0x540b6100,
            0x540b6500u32,
            processing_hooks.as_slice(),
            [].as_slice(),
        ),
        (
            &menu,
            0x540b6500,
            0x540b9800u32,
            menu_hooks.as_slice(),
            menu_instructions.as_slice(),
        ),
        (
            &framing,
            0x540b9800,
            0x540be000u32,
            framing_hooks.as_slice(),
            framing_instructions.as_slice(),
        ),
    ] {
        assert_eq!(
            component.patches.len(),
            expected.len() + instructions.len() + 1
        );
        for (patch, &(address, before, after)) in component.patches[expected.len() + 1..]
            .iter()
            .zip(instructions)
        {
            assert_eq!(patch.address, address);
            assert_eq!(patch.before, hex::decode(before)?);
            assert_eq!(patch.after, hex::decode(after)?);
        }
        let code = &component.patches[0];
        assert_eq!(code.address, base);
        assert_eq!(code.after.len(), component.code_size);
        assert!(!code.after.is_empty());
        assert!(u64::from(base) + code.after.len() as u64 <= u64::from(end));
        assert_eq!(code.before, vec![0; code.after.len()]);

        let relocations: Vec<_> = code
            .after
            .chunks_exact(4)
            .enumerate()
            .filter_map(|(index, word)| (word == [0x40, 0x23, 0x01, 0x58]).then_some(index * 4))
            .collect();
        assert!(!relocations.is_empty());
        assert_eq!(component.relocations, relocations);

        for (patch, &(address, symbol, before, link)) in component.patches[1..].iter().zip(expected)
        {
            assert_eq!(patch.address, address, "{symbol}");
            assert_eq!(patch.before, hex::decode(before)?, "{symbol}");
            assert_eq!(patch.after.len(), 4, "{symbol}");
            let instruction = u32::from_le_bytes(patch.after.as_slice().try_into()?);
            assert_eq!(
                instruction >> 24,
                if link { 0xeb } else { 0xea },
                "{symbol}"
            );
            // A32 branches use a signed 24-bit word displacement from PC + 8.
            let displacement = ((instruction << 8) as i32 >> 6) as i64;
            let destination = i64::from(address) + 8 + displacement;
            let offset = component.symbols[symbol];
            assert!(offset < component.code_size as u32, "{symbol}");
            assert_eq!(offset % 4, 0, "{symbol}");
            assert_eq!(destination, i64::from(base) + i64::from(offset), "{symbol}");
        }
    }
    Ok(())
}

#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE and ARM toolchain"]
fn pinned_firmware_build_is_reproducible() -> Result<()> {
    let firmware =
        PathBuf::from(std::env::var("GRKIT_TEST_FIRMWARE").context("Set GRKIT_TEST_FIRMWARE")?);
    let tools = tools()?;
    let body = verify_firmware(&firmware, &target::GR3_210)?;
    let directory = tempfile::tempdir()?;
    let first = build_image(&body, &directory.path().join("first"), &tools)?;
    let second = build_image(&body, &directory.path().join("second"), &tools)?;
    assert_eq!(first.files, second.files);
    assert_eq!(first.manifest, second.manifest);
    Ok(())
}

/// Resolves the relocation tables of `objects/module_payload.h` against an RTOS
/// image: the result line, then each symbol as (pinned, resolved).
fn resolve_image(output: &Path, image: &[u8]) -> Result<(String, Vec<(u32, u32)>)> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rtos.bin");
    fs::write(&path, image)?;
    let executable = directory.path().join("relocation_image");
    let compiler = std::env::var("GRKIT_HOST_CC").unwrap_or_else(|_| "cc".into());
    let objects = output.join("objects");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = std::process::Command::new(compiler)
        .args(["-std=c11", "-O2", "-w", "-I"])
        .arg(objects.join("runtime"))
        .arg("-I")
        .arg(&objects)
        .arg(root.join("tests/runtime/relocation_image.c"))
        .arg("-o")
        .arg(&executable)
        .status()?;
    ensure!(status.success(), "Compile the relocation harness");
    let output = std::process::Command::new(&executable)
        .arg(&path)
        .output()?;
    ensure!(output.status.success(), "Run the relocation harness");
    let text = String::from_utf8(output.stdout)?;
    let mut lines = text.lines();
    let result = lines.next().context("No result")?.to_owned();
    let hex = |value: &str| u32::from_str_radix(value.trim_start_matches("0x"), 16);
    let symbols = lines
        .filter_map(|line| line.strip_prefix("symbol "))
        .map(|line| {
            let (pinned, resolved) = line.split_once(' ').context("Invalid symbol line")?;
            Ok((hex(pinned)?, hex(resolved)?))
        })
        .collect::<Result<_>>()?;
    Ok((result, symbols))
}

#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4 and ARM toolchain"]
fn gr4_relocation_resolves_other_builds() -> Result<()> {
    let firmware = PathBuf::from(
        std::env::var("GRKIT_TEST_FIRMWARE_GR4").context("Set GRKIT_TEST_FIRMWARE_GR4")?,
    );
    let tools = tools()?;
    let body = verify_firmware(&firmware, &target::GR4_111)?;
    let directory = tempfile::tempdir()?;
    let built = build_image(&body, &directory.path().join("gr4"), &tools)?;
    // The pinned image resolves each symbol to its pinned address, so the
    // installer changes nothing on it.
    let (result, symbols) = resolve_image(&built.output, &body.body)?;
    assert_eq!(result, "ok 0x543d3000");
    assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
    // GR IV 1.04, another build with other addresses, if given.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE_GR4_104") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, symbols) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert_eq!(result, "ok 0x543d2000");
        let resolved: BTreeMap<_, _> = symbols.into_iter().collect();
        // FW_SETTINGS, a template getter found through its caller, and the
        // capture property singleton in the BSS.
        assert_eq!(resolved[&0x5323f018], 0x5323ef40);
        assert_eq!(resolved[&0x533b7110], 0x533b7004);
        assert_eq!(resolved[&0x5537960c], 0x553795fc);
    }
    // GR IV HDF also reports 1.11 for the February 18 build from issue #9.
    // This is a complete, unpatched RTOS capture, not an update container.
    // Keep the input local; these expected addresses were checked independently
    // while preparing a fixed-address runtime tested on that camera.
    if let Some(path) = std::env::var_os("GRKIT_TEST_RTOS_GR4_111_20260218") {
        let image = fs::read(path)?;
        ensure!(
            image.len() == 0x013d4140
                && firmware::sha256(&image)
                    == "60250bf7c497af13d6579fc58736cfddbffa32ffb4f8ff3a1403854b9da40d5e",
            "Expected the complete, unpatched GR IV HDF February 18 RTOS capture"
        );
        let (result, symbols) = resolve_image(&built.output, &image)?;
        assert_eq!(result, "ok 0x543d5000");
        let resolved: BTreeMap<_, _> = symbols.into_iter().collect();
        for (pinned, expected) in [
            (0x543d3000, 0x543d5000), // Tail after the larger image.
            (0x553b2ab0, 0x553b2ae0), // Script array count and table.
            (0x553b3ec4, 0x553b3ef4),
            (0x558372d4, 0x55837314), // BSS end / minimum heap address.
            (0x531bf9d8, 0x531bfad0), // ADJ touch and icon lookup hooks.
            (0x5323db68, 0x5323dc60),
            (0x533b7110, 0x533b72f0), // Parameter getter found through a caller.
            (0x5537960c, 0x5537962c), // Settings singleton.
            (0x5500211c, 0x5500211c), // English slot does not move.
            (0x5501adbc, 0x5501adc4), // Standard record and native tables.
            (0x55053780, 0x55053788),
            (0x55053504, 0x5505350c),
            (0x55052cf4, 0x55052cfc),
            (0x540498d8, 0x5404a888), // Four initialized Standard-record pointers.
            (0x54049a70, 0x5404aa20),
            (0x54049b28, 0x5404aad8),
            (0x54049518, 0x5404a4c8),
        ] {
            assert_eq!(
                resolved[&pinned], expected,
                "February-build relocation of {pinned:#x}"
            );
        }
    }
    // GR III is not a build of this version: nothing resolves.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, _) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert!(result.starts_with("failed"));
    }
    Ok(())
}

#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE and ARM toolchain"]
fn gr3_relocation_resolves_its_builds() -> Result<()> {
    let firmware =
        PathBuf::from(std::env::var("GRKIT_TEST_FIRMWARE").context("Set GRKIT_TEST_FIRMWARE")?);
    let tools = tools()?;
    let body = verify_firmware(&firmware, &target::GR3_210)?;
    let directory = tempfile::tempdir()?;
    let built = build_image(&body, &directory.path().join("gr3"), &tools)?;
    let (result, symbols) = resolve_image(&built.output, &body.body)?;
    assert_eq!(result, "ok 0x540b6000");
    assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
    // GR IIIx 1.60 runs the RTOS code of GR III 2.10, if given.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE_GR3X") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, symbols) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert_eq!(result, "ok 0x540b6000");
        assert!(symbols.iter().all(|(pinned, resolved)| pinned == resolved));
    }
    // GR IV is not a build of this version: nothing resolves.
    if let Ok(path) = std::env::var("GRKIT_TEST_FIRMWARE_GR4") {
        let decoded = firmware::decode(&fs::read(path)?)?;
        let (result, _) = resolve_image(&built.output, firmware::rtos_section(&decoded)?)?;
        assert!(result.starts_with("failed"));
    }
    Ok(())
}

/// The built-in film looks on the camera's Standard base gamma: as the exposure of a color
/// grows from -2 EV up to where the sensor clips, its L* may not fall. The hue sectors can darken or saturate bright
/// colors; the fit bounds this (`fit::sectors::Ramps`), and refits must keep it bounded.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE"]
fn builtin_effects_brighten_with_exposure() -> Result<()> {
    use grkit_core::cube::{self, LabConverter};
    use grkit_core::fit::Model;

    let firmware =
        PathBuf::from(std::env::var("GRKIT_TEST_FIRMWARE").context("Set GRKIT_TEST_FIRMWARE")?);
    let image = verify_firmware(&firmware, &target::GR3_210)?;
    let model = Model::new(&firmware::standard_base_gamma(&image)?);
    let lab = LabConverter::default();
    let effects = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../effects");
    let mut falls = Vec::new();
    for entry in fs::read_dir(effects)? {
        let path = entry?.path().join("effect.json");
        if !path.is_file() {
            continue;
        }
        let definition = grkit_core::effects::parse_json(&fs::read(&path)?)?;
        let (effect, _) =
            grkit_core::effects::decode(&grkit_core::effects::compile_definition(&definition)?)?;
        let params = grkit_core::site::params(&effect);
        let mut worst = 0.0f64;
        for k in 0..36 {
            let hue = std::f64::consts::TAU * k as f64 / 36.0;
            for (l, chroma) in [(30.0, 40.0), (50.0, 50.0), (70.0, 40.0), (40.0, 70.0)] {
                let base = cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]);
                if base.iter().any(|v| !(0.0..=1.0).contains(v)) {
                    continue;
                }
                let mut peak = f64::MIN;
                for step in 0..=48 {
                    let exposure = 2f64.powf(-2.0 + 0.125 * f64::from(step));
                    let linear = base.map(|v| v * exposure);
                    // The sensor clips about 3 times above diffuse white.
                    if linear.iter().any(|v| *v > 3.0) {
                        break;
                    }
                    let lightness = lab.lab(model.render(&params, linear))[0];
                    peak = peak.max(lightness);
                    worst = worst.max(peak - lightness);
                }
            }
        }
        falls.push((
            definition["effect_id"].as_str().unwrap_or("?").to_string(),
            worst,
        ));
    }
    falls.sort_by(|a, b| b.1.total_cmp(&a.1));
    eprintln!("largest L* falls: {falls:.2?}");
    assert_eq!(falls.len(), 16);
    // The fits before the lightness bound fell by up to 30 L*.
    assert!(falls[0].1 < 1.0, "{falls:.2?}");
    Ok(())
}

// Fixed contracts captured from the retired implementation using synthetic data only.
// These checks deliberately do not derive their expected bytes from the Rust producer.
#[test]
fn generated_header_matches_fixed_synthetic_contract() -> Result<()> {
    let plan = PatchPlan {
        patches: vec![
            Patch {
                address: 0x53002080,
                points: 0,
                before: vec![0; 8],
                after: hex::decode("4023015801020304")?,
                site: None,
            },
            Patch {
                address: 0x531a48f0,
                points: 0,
                before: hex::decode("f431d0e5")?,
                after: hex::decode("000000ea")?,
                site: None,
            },
        ],
        relocations: vec![(0, 0)],
        references: Vec::new(),
    };
    validate_plan(&plan)?;
    let header = loader::header(&plan, &synthetic_firmware(), TEST_VERSION)?;
    // The installer names its runtime build for the package check.
    assert!(header.starts_with(
        "static const char target_marker[] __attribute__((used)) = \"GRKIT_TARGET=gr3-2.10\";\n"
    ));
    assert_eq!(
        sha256(header.as_bytes()),
        "66d189c8a5affafc6e6237be5fdda7ae9b213be467337846130db02fbdf83eb9"
    );
    Ok(())
}

#[test]
fn generated_header_marks_only_rtos_tail_patches_as_unchecked() -> Result<()> {
    let plan = PatchPlan {
        patches: vec![
            Patch {
                address: 0x53002080,
                points: 0,
                before: vec![0; 8],
                after: hex::decode("4023015801020304")?,
                site: None,
            },
            Patch {
                address: components::TAIL_BASE,
                points: 0,
                before: vec![0; 4],
                after: vec![1, 2, 3, 4],
                site: None,
            },
        ],
        relocations: vec![(0, 0)],
        references: Vec::new(),
    };
    let header = loader::header(&plan, &synthetic_firmware(), TEST_VERSION)?;
    assert!(header.contains("{0x53002080,8,0,before_0,after_0},"));
    assert!(header.contains("{0x540b6000,4,1,before_1,after_1},"));
    Ok(())
}

#[test]
fn every_code_region_lies_in_the_tail() {
    for target in target::TARGETS {
        let layout = target.layout.unwrap();
        let (tail, end) = layout.tail;
        // The decoded image ends where the startup .data copy source ends.
        assert!(
            tail >= target.data_source + target.data_size,
            "{}",
            target.name
        );
        let mut regions: Vec<_> = [layout.processing, Some(layout.menu), layout.framing]
            .into_iter()
            .flatten()
            .collect();
        regions.sort();
        let mut next = tail;
        for (start, stop) in regions {
            assert!(
                start >= next && start < stop && stop <= end,
                "{}",
                target.name
            );
            next = stop;
        }
    }
}

#[test]
fn generated_header_gives_the_points_of_each_patch() -> Result<()> {
    let plan = PatchPlan {
        patches: vec![Patch {
            address: components::TAIL_BASE,
            points: 0,
            before: vec![0; 4],
            after: vec![1, 2, 3, 4],
            site: None,
        }],
        relocations: vec![],
        references: Vec::new(),
    };
    let header = loader::header(&plan, &synthetic_firmware(), TEST_VERSION)?;
    assert!(header.contains("{0x540b6000,4,1,before_0,after_0},"));
    // Code in the tail belongs to no extension point. Image Control needs the
    // menu, tuning, and picture encode points (bits 0, 1, and 4).
    assert!(header.contains("static const u32 patch_points[] = {0x0};"));
    assert!(header.contains("static const u32 plugin_points[] = {0x13,0xd,0x5,0x11,0x21};"));
    Ok(())
}

#[test]
fn menu_assembly_symbols_are_typed_functions() {
    // Thumb menu C reaches these A32 symbols with BL. Without a function type,
    // the linker keeps BL instead of BLX and the call runs A32 code as Thumb.
    let source = include_str!("../../../runtime/menu.S");
    let lines: Vec<_> = source.lines().map(str::trim).collect();
    let mut globals = 0;
    for (index, line) in lines.iter().enumerate() {
        let Some(name) = line.strip_prefix(".global ") else {
            continue;
        };
        globals += 1;
        assert_eq!(
            lines.get(index + 1).copied(),
            Some(format!(".type {name}, %function").as_str()),
            "{name}"
        );
    }
    assert!(globals > 0);
}

#[test]
fn production_scripts_and_activation_match_fixed_contracts() {
    let files = grkit_core::runtime::runtime_files(b"ELF");
    let expected = [
        (
            "SCRIPT/STARTUP.TTL",
            "bfa9f28e54f87af7a8191d915365c1617f533f1fdd2d6fcc47d0f549fe51083e",
        ),
        (
            "SCRIPT/PLUGINS/PLUGINS.TTL",
            "6c701c20bfdcee3a6d336800894b7462b59dd86420eecd51483ab4abc7b793d6",
        ),
        (
            "SCRIPT/PLUGINS/INSTALL.BIN",
            "706abe3c90152075e656b661079730facf323f3ebccda7547ee1935c90845a09",
        ),
        (
            "SCRIPT/PLUGINS/SCAN.SH",
            "69529aeb6e5201c703cc0a2ef5d58ad48c0b473da84c75e242a74a0462ce8c86",
        ),
        (
            "SCRIPT/PLUGINS/INSTALL.SH",
            "eaa25cde3b7a957eae4de653150343abc556cbce826513fc5e70ed3bd95fe9af",
        ),
    ];
    assert_eq!(files.len(), expected.len());
    for (path, hash) in expected {
        assert_eq!(sha256(&files[path]), hash, "{path}");
    }
    assert!(!files
        .values()
        .any(|data| data.windows(8).any(|part| part == b"--select")));
    // Each camera gets the same files under its own ModeSet name.
    for (camera, modeset) in [
        (&grkit_core::camera::GR3, "00078350.588"),
        (&grkit_core::camera::GR3X, "00078490.609"),
        (&grkit_core::camera::GR4, "00078560.636"),
    ] {
        let activation = grkit_core::runtime::activation_files(camera);
        assert_eq!(activation.len(), 2);
        assert_eq!(
            sha256(&activation[modeset]),
            "94db9a80a0e5c8f6bd8f3fce41aff6b8758f58333193ed48eb0b6db7db76afb3"
        );
        assert_eq!(
            sha256(&activation["DEVELOP.MOD"]),
            "0d622b3b4e26b183a0434f876f33cf56787c9b37c9b05e850f4be4a15c8ccf2d"
        );
    }
}

#[test]
fn each_camera_offers_what_its_runtime_has() {
    for (camera, name) in grkit_core::camera::CAMERAS
        .into_iter()
        .flat_map(|camera| camera.targets().into_iter().map(move |name| (camera, name)))
    {
        let target = target::find(name).unwrap();
        let layout = target.layout.unwrap();
        assert_eq!(layout.processing.is_some(), camera.effects, "{}", camera.id);
        // Each point of each plugin that the camera offers has hooks.
        let points: Vec<_> = components::hook_sites(layout)
            .map(|(_, symbol)| components::point_of(symbol).unwrap())
            .collect();
        for plugin in &grkit_core::plugins::PLUGINS {
            if camera.plugins.contains(&plugin.id) {
                for point in plugin.points {
                    assert!(
                        points.contains(point),
                        "{} {} {point:?}",
                        camera.id,
                        plugin.id
                    );
                }
            }
        }
    }
}

#[test]
fn each_point_has_its_hooks() {
    // The hooks of each point. The in-place patches add 1 to tuning, and on
    // GR III 5 to capture.aspect (build.json "points" counts both).
    for target in target::TARGETS {
        let expected = match target.family {
            "gr3" => [30, 48, 5, 9, 1, 2],
            "gr4" => [29, 50, 5, 11, 1, 4],
            family => panic!("No hook counts for the {family} family"),
        };
        let mut counts = [0; 6];
        for (_, symbol) in components::hook_sites(target.layout.unwrap()) {
            counts[components::point_of(symbol).unwrap() as usize] += 1;
        }
        assert_eq!(counts, expected, "{}", target.name);
    }
    assert!(components::point_of("unknown_hook").is_err());
}

#[test]
#[ignore = "Requires ARM-capable Clang and LLD; run with --ignored"]
fn dump_tool_is_a_static_arm_linux_program() -> Result<()> {
    let files = dump::files(&tools()?)?;
    let names: Vec<_> = files.keys().map(String::as_str).collect();
    let mut expected = grkit_core::packaging::DUMP_FILES.to_vec();
    expected.sort();
    assert_eq!(names, expected);
    let binary = &files["SD_ROOT/SCRIPT/GRDUMP/DUMP.BIN"];
    let half = |offset: usize| u16::from_le_bytes([binary[offset], binary[offset + 1]]);
    let word = |offset: usize| u32::from_le_bytes(binary[offset..offset + 4].try_into().unwrap());
    // ELF32, little endian, an executable for ARM.
    assert_eq!(&binary[..6], b"\x7fELF\x01\x01");
    assert_eq!((half(16), half(18)), (2, 40));
    // No program header asks for an interpreter, so the camera needs no libc.
    let (table, size, count) = (word(28) as usize, half(42) as usize, half(44) as usize);
    assert!((0..count).all(|index| word(table + index * size) != 3));
    assert!(binary.windows(8).any(|part| part == b"/dev/mem"));
    // The script runs the program from the folder where the program writes.
    let script = std::str::from_utf8(&files["SD_ROOT/SCRIPT/GRDUMP/DUMP.SH"])?;
    assert!(script.starts_with("/media/storage1/SCRIPT/GRDUMP/DUMP.BIN\n"));
    Ok(())
}

/// The definitions of a target header and of the headers that it includes from
/// `runtime/targets/`, by name. `own` keeps only those of the target header.
fn header_definitions(path: &str, own: bool) -> BTreeMap<String, String> {
    let source = runtime_source(path).unwrap_or_else(|| panic!("No runtime/{path}"));
    let text = std::str::from_utf8(source).unwrap().replace("\\\n", " ");
    let mut definitions = BTreeMap::new();
    for line in text.lines() {
        if let Some(include) = line.strip_prefix("#include \"") {
            if !own {
                let include = include.trim_end_matches('"');
                definitions.extend(header_definitions(&format!("targets/{include}"), false));
            }
        } else if let Some(definition) = line.strip_prefix("#define ") {
            let (name, value) = definition.split_once(' ').unwrap_or((definition, ""));
            // Skip the include guards.
            if !(name.starts_with("GRKIT_") && name.ends_with("_H")) {
                definitions.insert(name.to_string(), value.trim().to_string());
            }
        }
    }
    definitions
}

/// The hook symbols of each table of a layout, with their flags, in order.
fn hook_symbols(layout: &components::Layout) -> Vec<(&'static str, Vec<(&'static str, bool)>)> {
    let hooks = |table: &[components::Hook]| table.iter().map(|&(_, s, _, f)| (s, f)).collect();
    vec![
        (
            "processing",
            layout
                .processing_hooks
                .iter()
                .map(|&(_, s, f)| (s, f))
                .collect(),
        ),
        (
            "menu",
            layout
                .menu_hooks
                .iter()
                .map(|&(_, s, _)| (s, false))
                .collect(),
        ),
        ("viewfinder", hooks(layout.viewfinder_hooks)),
        ("metadata", hooks(layout.metadata_hooks)),
        ("parameter", hooks(layout.parameter_hooks)),
        ("settings", hooks(layout.settings_hooks)),
        ("framing", hooks(layout.framing_hooks)),
        ("picture", hooks(layout.picture_hooks)),
    ]
}

/// Definitions that only some builds of a family have, because the code of
/// those builds differs. The runtime tests each with `#ifdef`.
const BUILD_ONLY_DEFINITIONS: &[(&str, &[&str])] = &[(
    "gr4-1.20",
    &["NATIVE_ICON_LIMIT", "NATIVE_MONOCHROME_TYPES_HIGH"],
)];

#[test]
fn builds_of_a_family_have_the_same_hooks_and_definitions() {
    for target in target::TARGETS {
        let first = target::TARGETS
            .iter()
            .find(|other| other.family == target.family)
            .unwrap();
        let (layout, first_layout) = (target.layout.unwrap(), first.layout.unwrap());
        for ((table, symbols), (_, expected)) in hook_symbols(layout)
            .into_iter()
            .zip(hook_symbols(first_layout))
        {
            assert_eq!(
                symbols, expected,
                "The {table} hooks of {} differ from those of {}",
                target.name, first.name
            );
        }
        assert_eq!(
            (
                layout.contexts.len(),
                layout.instruction_patches.len(),
                layout.framing_patches.len()
            ),
            (
                first_layout.contexts.len(),
                first_layout.instruction_patches.len(),
                first_layout.framing_patches.len()
            ),
            "{}",
            target.name
        );
        let names = |target: &target::Target| -> BTreeSet<String> {
            let only: &[&str] = BUILD_ONLY_DEFINITIONS
                .iter()
                .find(|(name, _)| *name == target.name)
                .map_or(&[], |(_, names)| names);
            header_definitions(&target.header(), true)
                .into_keys()
                .filter(|name| !only.contains(&name.as_str()))
                .collect()
        };
        assert_eq!(
            names(target),
            names(first),
            "The header of {} defines other names than that of {}",
            target.name,
            first.name
        );
    }
}

#[test]
fn each_target_has_a_header_a_camera_release_and_an_icon_stride() {
    let selector = std::str::from_utf8(runtime_source("firmware.h").unwrap()).unwrap();
    for target in target::TARGETS {
        let layout = target.layout.unwrap();
        let define = layout.define.strip_prefix("-D").unwrap();
        assert!(
            selector.contains(&format!(
                "defined({define})\n#include \"{}\"",
                target.header()
            )),
            "runtime/firmware.h does not select {} with {define}",
            target.header()
        );
        let shared = format!("targets/{}.h", target.family);
        let family = target::TARGETS
            .iter()
            .filter(|other| other.family == target.family);
        if family.count() > 1 {
            let header = runtime_source(&shared)
                .unwrap_or_else(|| panic!("The {} family has no {shared}", target.family));
            // `grkit port` reads the addresses of the build header only.
            assert!(
                !std::str::from_utf8(header).unwrap().contains("FIRMWARE(0x"),
                "{shared} names a firmware address; put it in the build header"
            );
        }
        assert!(
            grkit_core::camera::CAMERAS
                .iter()
                .any(|camera| camera.runs(target.name)),
            "No camera release runs {}",
            target.name
        );
        // Without a definition, abi.h gives the GR III stride.
        let stride = header_definitions(&target.header(), false)
            .remove("GRKIT_ICON_STRIDE")
            .unwrap_or_else(|| grkit_core::runtime::ICON_STRIDE.to_string());
        assert_eq!(
            grkit_core::runtime::icon_stride(target.name)
                .unwrap()
                .to_string(),
            stride,
            "{}",
            target.name
        );
    }
    for camera in grkit_core::camera::CAMERAS {
        for release in camera.releases {
            assert!(target::find(release.target).is_ok(), "{}", release.target);
        }
    }
}

/// An RTOS image of `size` bytes whose startup literals give `.data` at
/// `data_source` with `data_size` bytes, and `words` at their addresses.
fn rtos_image(size: usize, data_source: u32, data_size: u32, words: &[(u32, u32)]) -> Vec<u8> {
    let mut image = vec![0; size];
    let mut put = |address: u32, word: u32| {
        let at = (address - target::RTOS_BASE) as usize;
        image[at..at + 4].copy_from_slice(&word.to_le_bytes());
    };
    let literals = [
        data_source,
        relocation::DATA_START,
        data_size,
        0x5510_0000,
        0x5520_0000,
    ];
    for (index, word) in literals.into_iter().enumerate() {
        put(relocation::STARTUP_LITERALS + index as u32 * 4, word);
    }
    for &(address, word) in words {
        put(address, word);
    }
    image
}

#[test]
fn resolution_finds_moved_windows_and_needs_agreeing_sources() -> Result<()> {
    use relocation::{mask, Plan, Region, Source, Symbol, Window};
    // A function at 0x53001000 in the plan, at 0x53041000 in the other
    // image: it moved by four times the installer radius. Its words: a
    // prologue, a BL to 0x53001100 (+0x40000), a literal, MOVW and MOVT of
    // 0x5537abcd, and a return.
    let delta = 0x40000;
    let function = |base: u32, moved: u32| {
        vec![
            (base, 0xe92d_4010),
            (base + 4, 0xeb00_003d), // BL base + 0x100
            (base + 8, 0x5300_1234 + moved),
            (base + 12, 0xe30a_0bcd), // MOVW r0, #0xabcd
            (base + 16, 0xe345_0537), // MOVT r0, #0x5537
            (base + 20, 0xe8bd_8010),
        ]
    };
    let words: Vec<u32> = function(0x5300_1000, 0)
        .iter()
        .map(|&(_, word)| mask(word))
        .collect();
    let other = rtos_image(
        0x60000,
        0x5305_0000,
        0x100,
        &function(0x5300_1000 + delta, delta),
    );
    let window = |start, words: &[u32]| Window {
        region: Region::Image,
        start,
        words: words.to_vec(),
        search: true,
        probe: 0,
    };
    // Window 1 has code that the other image lacks.
    let missing = [words.as_slice(), &[0xe1a0_0000]].concat();
    let symbol = |address, sources| Symbol { address, sources };
    let plan = Plan {
        windows: vec![window(0x5300_1000, &words), window(0x5300_2000, &missing)],
        symbols: vec![
            symbol(
                0x5300_1008,
                vec![Source::Window {
                    window: 0,
                    offset: 8,
                }],
            ),
            symbol(
                0x5300_1100,
                vec![Source::Branch {
                    window: 0,
                    index: 1,
                    verify: None,
                }],
            ),
            symbol(
                0x5300_1234,
                vec![Source::Literal {
                    window: 0,
                    index: 2,
                }],
            ),
            symbol(
                0x5537_abcd,
                vec![Source::Pair {
                    window: 0,
                    movw: 3,
                    movt: 4,
                }],
            ),
            // A second source that gives another address.
            symbol(
                0x5300_1004,
                vec![
                    Source::Window {
                        window: 0,
                        offset: 4,
                    },
                    Source::Window {
                        window: 0,
                        offset: 8,
                    },
                ],
            ),
            // A missing window gives no value; the other source gives it.
            symbol(
                0x5300_1000,
                vec![
                    Source::Window {
                        window: 1,
                        offset: 0,
                    },
                    Source::Window {
                        window: 0,
                        offset: 0,
                    },
                ],
            ),
            symbol(
                0x5300_2000,
                vec![Source::Window {
                    window: 1,
                    offset: 0,
                }],
            ),
            symbol(0x5305_1000, vec![Source::Tail]),
        ],
    };
    // The installer radius does not reach the moved function.
    let near = relocation::resolve(&plan, &other, relocation::RADIUS)?;
    assert!(near.symbols[0].as_ref().unwrap_err().contains("not found"));
    let far = relocation::resolve(&plan, &other, port::RADIUS)?;
    let value = |index: usize| far.symbols[index].as_ref().map(|(value, _)| *value);
    assert_eq!(value(0), Ok(0x5304_1008));
    assert_eq!(value(1), Ok(0x5304_1100));
    assert_eq!(value(2), Ok(0x5304_1234));
    assert_eq!(value(3), Ok(0x5537_abcd));
    assert!(value(4).unwrap_err().contains("disagree"));
    assert_eq!(far.symbols[5].as_ref().unwrap().1.len(), 1);
    assert_eq!(value(5), Ok(0x5304_1000));
    assert!(value(6).unwrap_err().contains("not found"));
    assert_eq!(value(7), Ok(0x5305_1000));
    assert_eq!(far.layout.tail, 0x5305_1000);
    Ok(())
}

#[test]
fn port_lists_the_addresses_of_each_target() -> Result<()> {
    for target in target::TARGETS {
        let addresses = port::addresses(target)?;
        let names: BTreeSet<_> = addresses.iter().map(|a| (a.kind, a.name.clone())).collect();
        assert_eq!(names.len(), addresses.len(), "{}", target.name);
        // Each hook of the layout is a site, and each header address is listed.
        assert_eq!(
            addresses
                .iter()
                .filter(|a| a.kind == port::Kind::Site && a.symbol.is_some())
                .count(),
            hook_symbols(target.layout.unwrap())
                .iter()
                .map(|(_, symbols)| symbols.len())
                .sum::<usize>(),
            "{}",
            target.name
        );
        let header = std::str::from_utf8(runtime_source(&target.header()).unwrap())?;
        assert_eq!(
            addresses
                .iter()
                .filter(|a| a.kind == port::Kind::Header)
                .count(),
            header.matches("FIRMWARE(0x").count(),
            "{}",
            target.name
        );
    }
    Ok(())
}

/// GR IV 1.11 ported to 1.20 gives no address that differs from the gr4-1.20
/// target without a review. The image of each build is enough; no toolchain.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE_GR4 and GRKIT_TEST_FIRMWARE_GR4_120"]
fn port_of_gr4_111_to_120_agrees_with_the_gr4_120_target() -> Result<()> {
    let path = |name: &str| -> Result<PathBuf> {
        Ok(PathBuf::from(
            std::env::var(name).with_context(|| format!("Set {name}"))?,
        ))
    };
    let base = verify_firmware(&path("GRKIT_TEST_FIRMWARE_GR4")?, &target::GR4_111)?;
    let other = verify_firmware(&path("GRKIT_TEST_FIRMWARE_GR4_120")?, &target::GR4_120)?;
    let port = port::port(&base, &other.body, port::RADIUS)?;
    let check = port::check(&port, &target::GR4_120)?;
    assert_eq!(check["differ"], json!([]), "{check:#}");
    // The type tables grew in 1.20, so their starts need a review.
    let review: Vec<_> = check["review"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert!(
        review.contains(&"target:image_control.tables[0]"),
        "{check:#}"
    );
    assert!(check["equal"].as_u64().unwrap() >= 130, "{check:#}");
    assert_eq!(port.layout.data_source, target::GR4_120.data_source);
    assert_eq!(port.layout.data_size, target::GR4_120.data_size);
    assert_eq!(port.layout.heap_min, target::GR4_120.heap_min);
    Ok(())
}

/// The value that an ARM MOV (immediate) or MOVW loads, and its register.
fn move_value(word: u32) -> Option<(u32, u32)> {
    let register = (word >> 12) & 0xf;
    if word & 0x0ff0_0000 == 0x0300_0000 {
        return Some((register, relocation::move_immediate(word)));
    }
    (word & 0x0fe0_0000 == 0x03a0_0000).then(|| {
        let rotate = ((word >> 8) & 0xf) * 2;
        (register, (word & 0xff).rotate_right(rotate))
    })
}

/// The Fn hint IDs of a build header are those that the native ADJ refresh of
/// its firmware gives to FW_GUIDE_SET_CONTENT: the label in r1, the icon in r3.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR4, and GRKIT_TEST_FIRMWARE_GR4_120"]
fn fn_hint_ids_are_those_of_the_native_adj_refresh() -> Result<()> {
    for (variable, target) in [
        ("GRKIT_TEST_FIRMWARE", &target::GR3_210),
        ("GRKIT_TEST_FIRMWARE_GR4", &target::GR4_111),
        ("GRKIT_TEST_FIRMWARE_GR4_120", &target::GR4_120),
    ] {
        let path = std::env::var(variable).with_context(|| format!("Set {variable}"))?;
        let image = verify_firmware(Path::new(&path), target)?;
        let definitions = header_definitions(&target.header(), false);
        let number = |name: &str| -> Result<u32> {
            let value = definitions[name]
                .trim_start_matches("FIRMWARE(")
                .trim_end_matches(')');
            Ok(u32::from_str_radix(value.trim_start_matches("0x"), 16)?)
        };
        let (setter, text, icon) = (
            number("FW_GUIDE_SET_CONTENT")?,
            number("FN_GUIDE_TEXT")?,
            number("FN_GUIDE_ICON")?,
        );
        let code = &image.body[..(target.data_source - target::RTOS_BASE) as usize];
        let words: Vec<u32> = code
            .chunks_exact(4)
            .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
            .collect();
        let calls = (0..words.len()).filter(|&index| {
            let address = target::RTOS_BASE + index as u32 * 4;
            relocation::branch_target(address, words[index]) == Some(setter)
                && (words[index] >> 24) & 0xf == 0xb
        });
        let matching = calls
            .filter(|&index| {
                let before = &words[index.saturating_sub(9)..index];
                let loads: Vec<_> = before.iter().filter_map(|&word| move_value(word)).collect();
                loads.contains(&(1, text)) && loads.contains(&(3, icon))
            })
            .count();
        assert!(
            matching > 0,
            "{}: no native Fn hint with {text:#x} and {icon:#x}",
            target.name
        );
    }
    Ok(())
}
