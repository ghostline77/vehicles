# Architecture and data

`vehicles` owns record/appearance discovery policy, classification, performance
calculations, render recipes, comparisons and static publication. `ghostline-red`
owns compiled TweakDB, archive/CR2W/asset/rig decoding and legacy curve evaluation.
`renderer` owns reusable assembly preparation, materials, GPU work and resume
receipts. Vehicles does not depend on `items` or the archived Python tools.

## Published entities

- Vehicle: exact decimal TweakDB ID, verified native name when available, label,
  class, manufacturer/type, requested entity/appearance, compiled properties,
  linked UI/engine/drive records, performance and visual-selection relationships.
- Selection: native entity resource plus exact appearance, related gameplay
  records, components, transforms, parent/control attachments, rig identities,
  native requested/effective mesh appearances and material/chunk bindings.
- Preview: selection and renderer/recipe identity, state, semantic views,
  relative media paths, dimensions, MIME type, byte length and SHA-256.
- Source: declared game build/language and fingerprints of the complete compiled
  cache, optional native name dictionary, archive payload index and schema.

One gameplay record can expose several selections. Multiple records can share a
selection without sharing performance or behavior. All appearances on referenced
entities are discoverable even when an individual record requests only one paint.
An entity/appearance identity does not assert that every record can spawn it.

## Package

```text
manifest.json
index.html
assets/{app.js,style.css}
data/catalogue.json                 source declarations and collection shards
data/{vehicles,selections}/*.json   bounded arrays of native entities
data/{coverage,binding-report,curves}.json
data/browser/catalogue.js          compact browsing/comparison data
data/browser/details/*.js          on-demand native details
media/images/<native-resource-id>/<appearance>/<size>px/<view>.png
```

Consumers follow published media/detail paths; they do not recover identity from
folder names. Numeric resource/TweakDB identities stay strings. Media folders use
reversible byte escaping for native appearance names.

The CLI searches, prints and compares records using these same JSON collections.
The browser uses hash routes and relative paths for both hosted and offline use.
There is no server API or runtime database requirement.

Coverage counts records, names, selections, binding resolution, nonvisual entries,
preview states and images separately. A successful build can contain explicit
blocked entries; those remain outstanding work. A rendered image is not proof of
complete game-equivalent materials or runtime behavior.
