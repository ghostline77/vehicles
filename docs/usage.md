# Generate, render and distribute a vehicle catalogue

## Generate

Use Rust 1.92+ and Node.js 20+. Run from this repository.
Replace `YOUR_GAME_VERSION` with the installed game's version label and supply
its complete TweakDB cache. `--tweak-names` is optional.

```powershell
cargo build --release --locked
./target/release/ghostline-vehicles.exe build --game 'path/to/Cyberpunk 2077' --tweakdb 'path/to/Cyberpunk 2077/r6/cache/tweakdb_ep1.bin' --tweak-names tweakdbstr.kark --game-version YOUR_GAME_VERSION --out dist/catalogue
./target/release/ghostline-vehicles.exe search --catalogue dist/catalogue Caliburn
./target/release/ghostline-vehicles.exe show --catalogue dist/catalogue Vehicle.v_sport1_rayfield_caliburn_player
./target/release/ghostline-vehicles.exe compare --catalogue dist/catalogue FIRST_RECORD_ID SECOND_RECORD_ID
```

Select one complete compiled cache. Base and expansion caches are not summed or
blindly overlaid. Native names are optional verified lookup hints; decimal native
IDs remain usable without a dictionary. The selected cache, name dictionary and
archive/schema identities are fingerprinted. The build reads unmodified base/EP1
archives and does not include runtime mod substitutions.

Every discovered selection finishes resolved or blocked. Partial component
details and errors remain in the catalogue and `data/binding-report.json`.
Metadata-only records remain searchable. Unspecified and random record appearance
requests retain their original meaning alongside enumerated authored selections.
No spawnability, player ownership or AI suitability is inferred from names.

## Render

Existing catalogues can be rendered against an updated game installation. Source
fingerprints record catalogue provenance; they do not require an identical game
build. New previews resolve native resources from the installed game and record
its identity, so previews from another installation are not reused as current.

```powershell
./target/release/ghostline-vehicles.exe plan-renders --catalogue dist/catalogue --size 1024 --out work/renders.json
./target/release/ghostline-vehicles.exe render-batch --catalogue dist/catalogue --game 'path/to/Cyberpunk 2077' --plan work/renders.json --work work/previews --workers 16 --cancel-file work/stop-rendering
```

Plans carry exact native selections, source identities, size and five semantic
views: hero, front, back, left and right. Edit a plan's selection list for a review
set, or use `--offset`/`--limit`. Omitting a range schedules the entire plan.
Blocked bindings remain in the plan's report rather than becoming guessed assets.

Catalogue binding resolution uses up to eight native workers with shared resource
documents and compact mesh-binding caches. Render workers share native source caches,
each retaining its own GPU renderer. Rendering
evaluates native attachments/rigs rather than flattening components into arbitrary
bind poses. Missing dependencies and unsupported materials are reported per
selection. `renderer` owns material implementations and rendering approximations.

Standalone recipes supply explicit entity-origin placement frames where a vehicle
normally receives its world placement externally, and select the owning native rig
for slot attachments. These constraints appear in preview inspection notes. Native
zero-scale geometry and shadow-only casters are omitted before dependency loading.
Known moved-resource corrections retain the requested resource alongside the
effective binding and apply only while the original is absent. Missing visible
meshes and absent authored appearances remain binding errors; they are never
replaced by an unrelated vehicle or a blank successful preview.
Five-view recipes fit projected geometry separately for each direction, so front
and rear views do not inherit the side view's empty space. Preview details publish
camera matrices, lighting and material approximation notes. Recipes require visual
validation for each native vehicle family.

Only the coordinator writes catalogue media. It commits receipts/progress per job,
publishes periodically and flushes on completion or cooperative cancellation.
Create the cancellation file to stop; remove it and repeat the command to resume.
Renderer receipts validate source/settings/build identities and PNG hashes.
Changed recipes invalidate old published views. `work/` holds preparation and
resume data; catalogue images belong in `dist/catalogue/media/`.

For the same native source snapshot, pass `render-batch --resume-from work/previews`
to restore published media from completed render receipts. The option is repeatable
for later repair batches. Visible native bindings, recipe settings and PNG hashes
are checked before reuse; unrelated physics/editor nodes do not invalidate images.

## Browse and distribute

Open `dist/catalogue/index.html` directly. Search native names/IDs, appearances,
manufacturers and types. Select appearances on cards, inspect technical detail
pages, compare up to four exact record/appearance selections and copy structured
native identities. Different paints of one record can be compared together.

```powershell
./target/release/ghostline-vehicles.exe verify --catalogue dist/catalogue
./target/release/ghostline-vehicles.exe package --catalogue dist/catalogue --out dist/offline
./target/release/ghostline-vehicles.exe package --catalogue dist/catalogue --out dist/hosted --media-base https://media.example.com/vehicles
```

Both editions include their referenced media. Hosted browsing supports relative
paths under a GitHub Pages subdirectory; an optional HTTPS media base can point
to R2 or a custom domain. The offline edition uses only its local media and data.
Browser data uses scripts and on-demand detail scripts to support `file://`;
equivalent JSON collections are available for agents.

See [architecture and data](architecture.md) and
[performance evidence](performance.md).
