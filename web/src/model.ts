import type { BrowserJson, BrowserMedia } from "@ghostline/ui-core";
export interface Vehicle {
  id: string;
  label: string;
  native_name?: string | null;
  description?: string | null;
  manufacturer?: string | null;
  vehicle_type?: string | null;
  requested_appearance?: string | null;
  entity?: BrowserJson;
  tags?: BrowserJson;
  selections: string[];
  diagnostics: string[];
  detail: string;
  performance?: {
    specifications?: Record<string, number | null>;
    calculated?: { top_gear?: Record<string, number | null> };
  };
}
export interface Selection {
  id: string;
  appearance: string | null;
  media: BrowserMedia[];
  preview_state: string;
  resolution: string;
  resource: BrowserJson;
  records: string[];
  detail: string;
}
export interface VehicleData {
  records: Vehicle[];
  selections: Selection[];
  sources: BrowserJson;
  coverage: Record<string, number>;
  media_base?: string;
}
declare global {
  var GHOSTLINE_VEHICLES: VehicleData | undefined;
  var GHOSTLINE_VEHICLE_DETAIL: BrowserJson | undefined;
}
export const format = (value: number | null | undefined, unit = ""): string =>
  typeof value === "number"
    ? new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(
        value,
      ) + unit
    : "Unavailable";
export const specs = (v: Vehicle): Record<string, number | null> =>
  v.performance?.specifications || {};
export const top = (v: Vehicle): Record<string, number | null> =>
  v.performance?.calculated?.top_gear || {};
export function filterVehicles(
  data: VehicleData,
  query: string,
  manufacturer: string,
  type: string,
  rendered: boolean,
  sort: string,
): Vehicle[] {
  const selections = new Map(data.selections.map((s) => [s.id, s])),
    q = query.trim().toLowerCase();
  return data.records
    .filter(
      (v) =>
        (!manufacturer || v.manufacturer === manufacturer) &&
        (!type || v.vehicle_type === type) &&
        (!rendered ||
          v.selections.some((id) => selections.get(id)?.media.length)) &&
        (!q ||
          [
            v.label,
            v.description,
            v.native_name,
            v.id,
            v.manufacturer,
            v.vehicle_type,
            JSON.stringify(v.tags),
            JSON.stringify(v.entity),
            ...v.selections.map((id) => selections.get(id)?.appearance),
          ]
            .join(" ")
            .toLowerCase()
            .includes(q)),
    )
    .sort(
      sort === "name"
        ? (a, b) => a.label.localeCompare(b.label)
        : (a, b) => {
            const value = (v: Vehicle) =>
              sort === "power"
                ? specs(v).horsepower
                : sort === "mass"
                  ? specs(v).physics_mass_kg
                  : top(v).hud_mph;
            return (
              (value(b) ?? -Infinity) - (value(a) ?? -Infinity) ||
              a.id.localeCompare(b.id)
            );
          },
    );
}
