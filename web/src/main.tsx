import { AppShell, Button, Input, Select, mountApp } from "@ghostline/ui-core";
import "./style.css";

function VehiclesCatalogue() {
  return (
    <AppShell product="Vehicles" homeHref="#/" project={<p id="coverage" />}>
      <main>
        <h1>Vehicles catalogue</h1>
        <form id="filters" role="search">
          <label>
            Search
            <Input
              id="search"
              type="search"
              placeholder="Vehicle, manufacturer, appearance or native ID"
            />
          </label>
          <label>
            Manufacturer
            <Select id="manufacturer">
              <option value="">All manufacturers</option>
            </Select>
          </label>
          <label>
            Type
            <Select id="type">
              <option value="">All types</option>
            </Select>
          </label>
          <label>
            Sort
            <Select id="sort">
              <option value="name">Name</option>
              <option value="power">Horsepower</option>
              <option value="speed">Calculated HUD ceiling</option>
              <option value="mass">Physics mass</option>
            </Select>
          </label>
          <label className="check">
            <Input id="rendered" type="checkbox" /> With renders
          </label>
        </form>
        <section id="comparison" hidden>
          <div className="section-title">
            <h2>Compare vehicles</h2>
            <Button id="clear-compare">Clear comparison</Button>
          </div>
          <div className="table-scroll">
            <table id="compare-table" />
          </div>
        </section>
        <section id="detail" hidden aria-label="Vehicle details" />
        <p id="status" role="status" />
        <section id="results" className="grid" aria-label="Vehicles" />
        <nav id="pagination" aria-label="Result pages">
          <Button id="previous">Previous</Button>
          <span id="page" />
          <Button id="next">Next</Button>
        </nav>
        <details className="source-notes">
          <summary>Sources, coverage and limitations</summary>
          <pre id="sources" />
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
      <dialog id="image-viewer">
        <Button id="close-image">Close</Button>
        <img alt="" />
        <p />
      </dialog>
      <footer>
        Ghostline vehicle catalogue · Native selections and published previews
      </footer>
    </AppShell>
  );
}
mountApp(<VehiclesCatalogue />, () => import("../app.js"));
