use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use ghostline_vehicles::{build, model::Catalogue, publish, render};
use serde_json::json;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(
    version,
    about = "Native vehicle catalogue generation, search, comparison and GPU previews"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Update browser assets without regenerating catalogue data or images.
    RefreshUi {
        #[arg(long)]
        catalogue: PathBuf,
    },
    /// Discover records and resolve every native visual selection without a GPU.
    Build {
        #[arg(long)]
        game: PathBuf,
        #[arg(long)]
        schema: Option<PathBuf>,
        #[arg(long)]
        tweakdb: PathBuf,
        #[arg(long)]
        tweak_names: Option<PathBuf>,
        #[arg(long, default_value = "en-us")]
        language: String,
        #[arg(long)]
        game_version: String,
        #[arg(long, default_value = "dist/catalogue")]
        out: PathBuf,
    },
    /// Search records and their exact visual selections; output JSON lines.
    Search {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
        query: String,
    },
    /// Print a record or exact native selection as JSON.
    Show {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
        id: String,
    },
    /// Compare gameplay records without merging performance by appearance.
    Compare {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
        #[arg(required = true)]
        ids: Vec<String>,
    },
    /// Establish five-view recipes and report blocked bindings.
    PlanRenders {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 1024)]
        size: u32,
    },
    /// Render planned selections concurrently and publish catalogue images.
    RenderBatch {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
        #[arg(long)]
        game: PathBuf,
        #[arg(long)]
        schema: Option<PathBuf>,
        #[arg(long)]
        plan: PathBuf,
        #[arg(long, default_value = "work/previews")]
        work: PathBuf,
        #[arg(long, default_value_t = 16)]
        workers: usize,
        #[arg(long)]
        cancel_file: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Restore completed previews from a same-source render cache (repeatable).
        #[arg(long)]
        resume_from: Vec<PathBuf>,
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Validate published identities, links, data files and image hashes.
    Verify {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
    },
    /// Produce the same browser and local media for hosted/offline distribution.
    Package {
        #[arg(long, default_value = "dist/catalogue")]
        catalogue: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "")]
        media_base: String,
    },
}

fn record<'a>(c: &'a Catalogue, id: &str) -> Option<&'a ghostline_vehicles::model::Vehicle> {
    c.vehicles.get(id).or_else(|| {
        c.vehicles
            .values()
            .find(|v| v.native_name.as_deref() == Some(id))
    })
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Build {
            game,
            schema,
            tweakdb,
            tweak_names,
            language,
            game_version,
            out,
        } => {
            let source = ghostline_renderer::GameSource::open(&game, schema.as_deref())?;
            source.validate_output(&out)?;
            let old = if out.join("manifest.json").exists() {
                Some(publish::read(&out)?)
            } else {
                None
            };
            let mut c = build::build(&build::Inputs {
                game: &game,
                schema: schema.as_deref(),
                tweakdb: &tweakdb,
                names: tweak_names.as_deref(),
                language: &language,
                game_version: &game_version,
            })?;
            // Official metadata can improve reflection without changing any game
            // payload. Verify the prior byte fingerprint before preserving images;
            // every individual selection must still have identical native bindings.
            use sha2::{Digest as _, Sha256};
            let schema_file_hash = source.schema_sha256().to_owned();
            let mut byte_identity = Sha256::new();
            byte_identity.update(source.archive_identity().as_bytes());
            byte_identity.update(schema_file_hash.as_bytes());
            let byte_identity = format!("{:x}", byte_identity.finalize());
            let unchanged_payloads = |o: &ghostline_vehicles::model::Catalogue| {
                o.sources
                    .iter()
                    .filter(|(k, _)| k.as_str() != "archives_schema_identity")
                    .eq(c
                        .sources
                        .iter()
                        .filter(|(k, _)| k.as_str() != "archives_schema_identity"))
                    && (o.sources.get("archives_schema_identity")
                        == c.sources.get("archives_schema_identity")
                        || o.sources.get("archives_schema_identity") == Some(&byte_identity))
            };
            if let Some(old) = old.filter(unchanged_payloads) {
                for (id, s) in &mut c.selections {
                    if let Some(prior) = old
                        .selections
                        .get(id)
                        .filter(|p| render::same_preview_inputs(p, s))
                    {
                        s.media = prior.media.clone();
                        s.recipe = prior.recipe.clone();
                        s.preview_state = prior.preview_state.clone();
                        s.diagnostics.extend(
                            prior
                                .diagnostics
                                .iter()
                                .filter(|d| d.starts_with("render: "))
                                .cloned(),
                        );
                    }
                }
            }
            publish::write(&c, &out, "")?;
            publish::prune_media(&c, &out)?;
            println!("{}", c.coverage());
        }
        Command::Search { catalogue, query } => {
            let c = publish::read(&catalogue)?;
            let q = query.to_lowercase();
            for v in c.vehicles.values() {
                let names = v
                    .selections
                    .iter()
                    .filter_map(|id| c.selections.get(id))
                    .map(|s| {
                        format!(
                            "{} {}",
                            s.resource,
                            s.appearance.as_deref().unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let haystack = format!(
                    "{} {} {} {} {} {}",
                    v.id,
                    v.native_name.as_deref().unwrap_or_default(),
                    v.label,
                    v.manufacturer.as_deref().unwrap_or_default(),
                    v.vehicle_type.as_deref().unwrap_or_default(),
                    names
                )
                .to_lowercase();
                if haystack.contains(&q) {
                    println!(
                        "{}",
                        json!({"id":v.id,"native_name":v.native_name,"label":v.label,"entity":v.entity,"selections":v.selections})
                    );
                }
            }
        }
        Command::Show { catalogue, id } => {
            let c = publish::read(&catalogue)?;
            let value = if let Some(v) = record(&c, &id) {
                serde_json::to_value(v)?
            } else {
                serde_json::to_value(c.selections.get(&id).context("unknown native identity")?)?
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Command::Compare { catalogue, ids } => {
            let c = publish::read(&catalogue)?;
            let values=ids.iter().map(|id|{let v=record(&c,id).with_context(||format!("unknown vehicle {id}"))?;Ok(json!({"id":v.id,"native_name":v.native_name,"label":v.label,"performance":v.performance,"selections":v.selections}))}).collect::<Result<Vec<_>>>()?;
            println!("{}", serde_json::to_string_pretty(&values)?);
        }
        Command::PlanRenders {
            catalogue,
            out,
            size,
        } => {
            let c = publish::read(&catalogue)?;
            let plan = render::plan(&c, size)?;
            publish::json_file(&out, &plan)?;
            println!(
                "{} jobs; {} blocked; {} nonvisual",
                plan.jobs.len(),
                plan.blocked.len(),
                plan.not_applicable.len()
            );
        }
        Command::RenderBatch {
            catalogue,
            game,
            schema,
            plan,
            work,
            workers,
            cancel_file,
            offset,
            resume_from,
            limit,
        } => {
            let mut c = publish::read(&catalogue)?;
            let source = ghostline_renderer::GameSource::open(&game, schema.as_deref())?;
            source.validate_output(&catalogue)?;
            source.validate_output(&work)?;
            for cache in resume_from {
                source.validate_output(&cache)?;
                render::restore_cached(&mut c, &catalogue, &cache)?;
            }
            let plan = serde_json::from_slice(&fs::read(plan)?)?;
            let cancel = cancel_file
                .map(ghostline_renderer::Cancellation::with_file)
                .unwrap_or_default();
            let report = render::run(
                &mut c,
                &source,
                &plan,
                &catalogue,
                &work,
                render::RunOptions {
                    workers,
                    cancel: &cancel,
                    offset,
                    limit,
                },
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            ensure!(
                !report["outcomes"]
                    .as_array()
                    .is_some_and(|v| v.iter().any(|v| v["state"] == "failed")),
                "render failures are recorded in the catalogue and work/render-report.json"
            );
        }
        Command::RefreshUi { catalogue } => publish::refresh_ui(&catalogue)?,
        Command::Verify { catalogue } => println!("{}", publish::verify(&catalogue)?),
        Command::Package {
            catalogue,
            out,
            media_base,
        } => {
            ensure!(
                media_base.is_empty() || media_base.starts_with("https://"),
                "hosted media base must use https"
            );
            publish::package(&catalogue, &out, &media_base)?;
            println!("{}", json!({"package":out,"media_base":media_base}));
        }
    }
    Ok(())
}
