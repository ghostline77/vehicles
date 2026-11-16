//! Inspect a native vehicle assembly with the catalogue's duplicate-prop policy.
use anyhow::{Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::{AssemblyOptions, Camera, GameSource, RenderOptions, Renderer, View};
use std::{fs, path::Path};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 3,
        "usage: render-assembly-sample GAME NEW_OUTPUT RESOURCE#APPEARANCE"
    );
    let source = GameSource::open(Path::new(&args[0]), None)?;
    let out = source.validate_output(Path::new(&args[1]))?;
    ensure!(!out.exists(), "output directory must be new");
    fs::create_dir_all(&out)?;
    let (key, appearance) = args[2]
        .split_once('#')
        .map_or((args[2].as_str(), None), |(a, b)| (a, Some(b)));
    let assembly = source.load_assembly(
        &ResourceKey::parse(key),
        appearance,
        &AssemblyOptions {
            omit_unbound_duplicates: true,
            ..Default::default()
        },
        &out.join("prepared"),
    )?;
    let options = RenderOptions {
        size: 1024,
        camera: Camera::FrameProjected(View::Hero),
    };
    fs::write(
        out.join("inspection.json"),
        serde_json::to_vec_pretty(&assembly.inspection(&[options])?)?,
    )?;
    let renderer = pollster::block_on(Renderer::new())?;
    let mut scene = renderer.prepare_scene(&assembly.scene)?;
    let image = renderer.render(&mut scene, options)?;
    image.image.save(out.join("hero.png"))?;
    println!(
        "{}",
        serde_json::json!({"gpu":image.adapter.name,"notes":assembly.preview_notes})
    );
    Ok(())
}
