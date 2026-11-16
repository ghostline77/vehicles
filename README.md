# Ghostline Vehicles

Search Cyberpunk 2077 vehicle records and exact native appearances, compare
performance, and browse multiview previews.

## Browse

Open a published catalogue's `index.html`. Keep its data and media together.
Browsing works without a server, installed game or extraction tools. Records that
share a model retain their separate performance data and identities.

## Generate and publish

Use the Rust CLI to build catalogue data from the installed game. Generation
resolves native bindings automatically and reports missing dependencies.
GPU preview rendering is a separate, resumable step.

- [Setup, generation, search and comparison](docs/usage.md#generate)
- [Render plans, cancellation and resume](docs/usage.md#render)
- [Verification, offline packaging and hosted media](docs/usage.md#browse-and-distribute)
- [Architecture and published data](docs/architecture.md)
- [Performance evidence](docs/performance.md)

Ghostline Red decodes native resources; Renderer owns preview rendering.
Catalogue bindings retain exact record/appearance identities. A rendered vehicle
does not establish that it can be spawned, owned by the player or driven by AI.
