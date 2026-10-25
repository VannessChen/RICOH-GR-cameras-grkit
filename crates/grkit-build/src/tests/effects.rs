use super::*;

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
    assert!(!falls.is_empty(), "The effects directory has no effect");
    // The fits before the lightness bound fell by up to 30 L*.
    assert!(falls[0].1 < 1.0, "{falls:.2?}");
    Ok(())
}
