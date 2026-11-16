import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  AppShell,
  Button,
  CopyJson,
  Input,
  NativeDetails,
  NativeGallery,
  Select,
  useHash,
} from "@ghostline/ui-core";
import { filterVehicles, format, specs, top } from "./model";
import type { Selection, Vehicle, VehicleData } from "./model";
import "./style.css";
type CompareEntry = { record: string; selection: string | null };
function Vehicles({ data }: { data: VehicleData }) {
  const [query, setQuery] = useState(""),
    [manufacturer, setManufacturer] = useState(""),
    [type, setType] = useState(""),
    [rendered, setRendered] = useState(false),
    [sort, setSort] = useState("name"),
    [page, setPage] = useState(0),
    [notice, setNotice] = useState("");
  const records = new Map(data.records.map((v) => [v.id, v])),
    selections = new Map(data.selections.map((s) => [s.id, s]));
  const [compared, setCompared] = useState<Map<string, CompareEntry>>(() => {
    try {
      const values: unknown = JSON.parse(
        localStorage.getItem("ghostline-vehicles-compare") || "[]",
      );
      return new Map(
        (Array.isArray(values) ? values : []).filter(
          (entry): entry is [string, CompareEntry] =>
            Array.isArray(entry) &&
            typeof entry[0] === "string" &&
            typeof entry[1]?.record === "string" &&
            records.has(entry[1].record),
        ),
      );
    } catch {
      return new Map();
    }
  });
  const [appearances, setAppearances] = useState<Record<string, string>>({});
  const hash = useHash(),
    rows = filterVehicles(data, query, manufacturer, type, rendered, sort),
    pages = Math.max(1, Math.ceil(rows.length / 36)),
    currentPage = Math.min(page, pages - 1);
  useEffect(() => {
    try {
      localStorage.setItem(
        "ghostline-vehicles-compare",
        JSON.stringify([...compared]),
      );
    } catch {
      /* Read-only/offline storage still permits browsing. */
    }
  }, [compared]);
  const current = (v: Vehicle): Selection | undefined =>
    v.selections
      .map((id) => selections.get(id))
      .find((s) => s?.id === appearances[v.id]) ||
    v.selections.map((id) => selections.get(id)).find((s) => s?.media.length) ||
    selections.get(v.selections[0] || "");
  const toggle = (v: Vehicle, s: Selection | undefined): void => {
    const key = JSON.stringify([v.id, s?.id ?? null]),
      next = new Map(compared);
    if (next.has(key)) next.delete(key);
    else if (next.size < 4)
      next.set(key, { record: v.id, selection: s?.id ?? null });
    else {
      setNotice("Compare up to four record/appearance selections.");
      return;
    }
    setCompared(next);
  };
  const card = (v: Vehicle) => {
    const s = current(v),
      options = v.selections.flatMap((id) =>
        selections.get(id) ? [selections.get(id)!] : [],
      );
    return (
      <article
        key={v.id}
        className="overflow-hidden rounded-xl border border-line bg-panel"
      >
        <NativeGallery
          key={s?.id || v.id}
          compact
          media={s?.media || []}
          label={s?.appearance || v.label}
          base={data.media_base || ""}
        />
        <div className="space-y-4 p-5">
          <p className="text-xs uppercase tracking-widest text-muted">
            {[v.manufacturer, v.vehicle_type].filter(Boolean).join(" / ")}
          </p>
          <h2 className="text-xl font-semibold">
            <a href={"#/vehicle/" + encodeURIComponent(v.id)}>{v.label}</a>
          </h2>
          <p className="break-all text-xs text-muted">
            {v.native_name || v.id}
          </p>
          {options.length > 0 && (
            <Select
              aria-label={`Appearance for ${v.label}`}
              value={s?.id || ""}
              onChange={(e) =>
                setAppearances({ ...appearances, [v.id]: e.target.value })
              }
            >
              {options.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.appearance || "Unspecified native appearance"}
                </option>
              ))}
            </Select>
          )}
          <dl className="grid grid-cols-2 gap-3 text-sm">
            {[
              ["Horsepower", format(specs(v).horsepower, " hp")],
              ["Physics mass", format(specs(v).physics_mass_kg, " kg")],
              ["Calculated HUD ceiling", format(top(v).hud_mph, " mph")],
              [
                "World-space gearing ceiling",
                format(top(v).world_kmh, " km/h"),
              ],
            ].map(([label, value]) => (
              <div key={label}>
                <dt className="text-xs text-muted">{label}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
          {(!v.requested_appearance || v.requested_appearance === "random") && (
            <p className="text-xs text-muted">
              {v.requested_appearance === "random"
                ? "The record selects its appearance at runtime; this preview is an explicit authored selection."
                : "No appearance specified by the record; choose an authored entity appearance."}
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => toggle(v, s)}>
              {compared.has(JSON.stringify([v.id, s?.id ?? null]))
                ? "Remove comparison"
                : "Compare"}
            </Button>
            {s && (
              <a
                className="rounded border border-line px-3 py-2 text-sm"
                href={"#/selection/" + encodeURIComponent(s.id)}
              >
                Appearance details
              </a>
            )}
          </div>
          <CopyJson
            label="Copy identity"
            value={{
              record_id: v.native_name || v.id,
              native_record_id: v.id,
              entity: v.entity,
              record_requested_appearance: v.requested_appearance,
              appearance: s?.appearance ?? null,
              source: data.sources,
            }}
          />
          {v.diagnostics.length > 0 && (
            <p className="text-xs text-warning">{v.diagnostics[0]}</p>
          )}
        </div>
      </article>
    );
  };
  const comparisonRows: [string, (v: Vehicle) => string][] = [
    ["Vehicle", (v) => `${v.label} (${v.native_name || v.id})`],
    [
      "Record appearance request",
      (v) => v.requested_appearance || "Unspecified",
    ],
    ["Horsepower", (v) => format(specs(v).horsepower, " hp")],
    ["UI mass", (v) => format(specs(v).ui_mass_kg, " kg")],
    ["Physics mass", (v) => format(specs(v).physics_mass_kg, " kg")],
    [
      "Calculated HUD ceiling",
      (v) =>
        `${format(top(v).hud_mph, " mph")} / ${format(top(v).hud_kmh, " km/h")}`,
    ],
    [
      "World-space gearing ceiling",
      (v) =>
        `${format(top(v).world_mph, " mph")} / ${format(top(v).world_kmh, " km/h")}`,
    ],
    ["Max RPM", (v) => format(specs(v).max_rpm)],
    ["Engine torque (native parameter)", (v) => format(specs(v).engine_torque)],
    [
      "Evidence",
      () =>
        "Authored specifications and calculated gearing ceilings; no gameplay measurements",
    ],
  ];
  const chosen = [...compared.values()].flatMap((e) => {
    const v = records.get(e.record);
    return v ? [{ v, s: selections.get(e.selection || "") }] : [];
  });
  let route: { kind: string; id: string } | undefined;
  try {
    const match = hash.match(/^#\/(vehicle|selection)\/(.*)$/);
    if (match) route = { kind: match[1]!, id: decodeURIComponent(match[2]!) };
  } catch {
    route = { kind: "invalid", id: "" };
  }
  const vehicle = route?.kind === "vehicle" ? records.get(route.id) : undefined,
    selection =
      route?.kind === "selection" ? selections.get(route.id) : undefined;
  return (
    <AppShell
      product="Vehicles"
      homeHref="#/"
      project={
        <p className="text-xs text-muted">
          {data.coverage.records?.toLocaleString()} records /{" "}
          {data.coverage.selections?.toLocaleString()} selections /{" "}
          {data.coverage.ready?.toLocaleString()} rendered /{" "}
          {data.coverage.blocked?.toLocaleString()} blocked
        </p>
      }
    >
      <main className="mx-auto w-full max-w-[1500px] space-y-6 p-4 sm:p-8">
        <h1 className="text-4xl font-semibold">Vehicles catalogue</h1>
        <form
          role="search"
          className="flex flex-wrap gap-4"
          onSubmit={(e) => e.preventDefault()}
        >
          <label className="grid min-w-56 flex-1 gap-2 text-sm">
            Search
            <Input
              type="search"
              placeholder="Vehicle, manufacturer, appearance or native ID"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setPage(0);
              }}
            />
          </label>
          {(
            [
              [
                "Manufacturer",
                manufacturer,
                setManufacturer,
                [
                  ...new Set(
                    data.records
                      .map((v) => v.manufacturer)
                      .filter((v): v is string => !!v),
                  ),
                ],
              ],
              [
                "Type",
                type,
                setType,
                [
                  ...new Set(
                    data.records
                      .map((v) => v.vehicle_type)
                      .filter((v): v is string => !!v),
                  ),
                ],
              ],
            ] as const
          ).map(([label, value, change, options]) => (
            <label key={label} className="grid gap-2 text-sm">
              {label}
              <Select
                value={value}
                onChange={(e) => {
                  change(e.target.value);
                  setPage(0);
                }}
              >
                <option value="">All</option>
                {[...options].sort().map((o) => (
                  <option key={o}>{o}</option>
                ))}
              </Select>
            </label>
          ))}
          <label className="grid gap-2 text-sm">
            Sort
            <Select
              value={sort}
              onChange={(e) => {
                setSort(e.target.value);
                setPage(0);
              }}
            >
              {[
                ["name", "Name"],
                ["power", "Horsepower"],
                ["speed", "Calculated HUD ceiling"],
                ["mass", "Physics mass"],
              ].map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={rendered}
              onChange={(e) => {
                setRendered(e.target.checked);
                setPage(0);
              }}
            />
            With renders
          </label>
        </form>
        {chosen.length > 0 && (
          <section aria-label="Compare vehicles" className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-semibold">Compare vehicles</h2>
              <Button onClick={() => setCompared(new Map())}>
                Clear comparison
              </Button>
            </div>
            <div className="overflow-auto">
              <table className="w-full text-left text-sm">
                <tbody>
                  <tr>
                    <th className="p-3">Appearance preview</th>
                    {chosen.map(({ v, s }, i) => (
                      <td key={i} className="min-w-64 p-3">
                        <NativeGallery
                          compact
                          media={s?.media || []}
                          label={s?.appearance || v.label}
                          base={data.media_base || ""}
                        />
                      </td>
                    ))}
                  </tr>
                  <tr>
                    <th className="p-3">Exact appearance</th>
                    {chosen.map(({ s }, i) => (
                      <td className="p-3" key={i}>
                        {s?.appearance || "Unspecified"}
                      </td>
                    ))}
                  </tr>
                  {comparisonRows.map(([label, get]) => (
                    <tr key={label} className="border-t border-line">
                      <th className="p-3">{label}</th>
                      {chosen.map(({ v }, i) => (
                        <td key={i} className="p-3">
                          {get(v)}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        )}
        {route && (
          <section
            aria-label="Vehicle details"
            className="space-y-5 rounded-xl border border-line p-5"
          >
            <a href="#/" className="text-accent">
              ← All vehicles
            </a>
            {vehicle ? (
              <>
                <h2 className="text-2xl font-semibold">{vehicle.label}</h2>
                <p>
                  {vehicle.description || "No localized description available"}
                </p>
                <div className="max-w-lg">{card(vehicle)}</div>
                <NativeDetails
                  path={vehicle.detail}
                  read={() => globalThis.GHOSTLINE_VEHICLE_DETAIL ?? null}
                  title="Native record, relationships and performance provenance"
                />
                {vehicle.selections
                  .map((id) => selections.get(id))
                  .filter((s): s is Selection => !!s)
                  .map((s) => (
                    <NativeDetails
                      key={s.id}
                      path={s.detail}
                      read={() => globalThis.GHOSTLINE_VEHICLE_DETAIL ?? null}
                      title={`${s.appearance || "Native selection"} · ${s.resolution} · ${s.preview_state}`}
                    />
                  ))}
              </>
            ) : selection ? (
              <>
                <h2 className="text-2xl font-semibold">
                  {selection.appearance || "Native vehicle selection"}
                </h2>
                <p className="break-all text-xs text-muted">{selection.id}</p>
                <div className="max-w-xl">
                  <NativeGallery
                    media={selection.media}
                    label={selection.appearance || selection.id}
                    base={data.media_base || ""}
                  />
                </div>
                <CopyJson
                  label="Copy selection"
                  value={{
                    resource: selection.resource,
                    appearance: selection.appearance,
                    associated_record_ids: selection.records,
                    source: data.sources,
                  }}
                />
                <p>
                  {selection.resolution} · {selection.preview_state}
                </p>
                {selection.records.map((id) => (
                  <p key={id}>
                    <a
                      href={"#/vehicle/" + encodeURIComponent(id)}
                      className="text-accent"
                    >
                      {records.get(id)?.label || id}
                    </a>
                  </p>
                ))}
                <NativeDetails
                  path={selection.detail}
                  read={() => globalThis.GHOSTLINE_VEHICLE_DETAIL ?? null}
                  title="Native components, transforms, materials and render provenance"
                />
              </>
            ) : (
              <p>Identity not found or invalid.</p>
            )}
          </section>
        )}
        <p role="status" className="text-muted">
          {notice ||
            `${rows.length.toLocaleString()} vehicle records · ${data.selections.length.toLocaleString()} distinct visual selections`}
        </p>
        <section
          aria-label="Vehicles"
          className="grid grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-3"
        >
          {rows.slice(currentPage * 36, (currentPage + 1) * 36).map(card)}
        </section>
        <nav
          aria-label="Result pages"
          className="flex items-center justify-center gap-4"
        >
          <Button
            disabled={currentPage === 0}
            onClick={() => setPage(currentPage - 1)}
          >
            Previous
          </Button>
          <span>
            Page {currentPage + 1} of {pages}
          </span>
          <Button
            disabled={currentPage + 1 >= pages}
            onClick={() => setPage(currentPage + 1)}
          >
            Next
          </Button>
        </nav>
        <details className="space-y-3 rounded border border-line p-4 text-sm text-muted">
          <summary>Sources, coverage and limitations</summary>
          <pre className="overflow-auto whitespace-pre-wrap break-all text-xs">
            {JSON.stringify(
              { sources: data.sources, coverage: data.coverage },
              null,
              2,
            )}
          </pre>
          <p>
            Performance specifications are authored values. Calculated speed
            ceilings are based on gearing and the main HUD curve; they are not
            measured sustained speeds. Native identities do not establish
            ownership, availability or verified spawnability.
          </p>
          <p>
            Blocked bindings and failed renders remain visible. Appearance
            previews are distinct from gameplay records and their performance.
          </p>
          <a href="data/coverage.json">Coverage JSON</a> ·{" "}
          <a href="data/catalogue.json">Catalogue JSON</a> ·{" "}
          <a href="data/binding-report.json">Binding diagnostics</a>
        </details>
      </main>
    </AppShell>
  );
}
const data = globalThis.GHOSTLINE_VEHICLES;
createRoot(document.getElementById("root")!).render(
  data ? (
    <Vehicles data={data} />
  ) : (
    <p role="alert">Catalogue data could not be loaded.</p>
  ),
);
