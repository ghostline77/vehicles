//! Render isolated native material fixtures without catalogue assembly dependencies.
use anyhow::{Result, ensure};
use ghostline_red::assets::ResourceKey;
use ghostline_renderer::{Camera, GameSource, Node, RenderOptions, Renderer, Scene, View};
use std::{fs, path::Path, sync::Arc};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() >= 3,
        "usage: render-material-sample GAME NEW_OUTPUT [--shared] RESOURCE..."
    );
    let source = GameSource::open(Path::new(&args[0]), None)?;
    let output = source.validate_output(Path::new(&args[1]))?;
    ensure!(!output.exists(), "output directory must be new");
    fs::create_dir_all(&output)?;
    let renderer = pollster::block_on(Renderer::new())?;
    let shared = args[2] == "--shared";
    let resources = &args[2 + usize::from(shared)..];
    ensure!(!resources.is_empty(), "at least one resource is required");
    for resource in resources {
        let (resource, appearance) = resource.split_once('#').unwrap_or((resource, "default"));
        let key = ResourceKey::parse(resource);
        let asset = if shared {
            let asset = source.load_shared_mesh_resource(&key, appearance)?;
            ensure!(
                Arc::ptr_eq(&asset, &source.load_shared_mesh_resource(&key, appearance)?),
                "prepared asset was not reused"
            );
            asset
        } else {
            Arc::new(source.load_mesh_resource(
                &key,
                appearance,
                &output.join(key.native_hash().to_string()),
            )?)
        };
        let notes = asset
            .material_notes()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut scene = Scene::new();
        let mesh = scene.add_shared_asset(asset);
        scene.add_node(Node::new("fixture", Some(mesh)))?;
        let mut prepared = renderer.prepare_scene(&scene)?;
        let result = renderer.render(
            &mut prepared,
            RenderOptions {
                size: 512,
                camera: Camera::FrameProjected(View::Hero),
            },
        )?;
        ensure!(
            result.image.pixels().any(|pixel| pixel.0[3] > 0),
            "fixture has no visible pixels"
        );
        result
            .image
            .save(output.join(format!("{}.png", key.native_hash())))?;
        fs::write(
            output.join(format!("{}.json", key.native_hash())),
            serde_json::to_vec_pretty(
                &serde_json::json!({"resource":resource,"gpu":result.adapter.name,"notes":notes}),
            )?,
        )?;
        println!("{resource}: rendered on {}", result.adapter.name);
    }
    Ok(())
}
