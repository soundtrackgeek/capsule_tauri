import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { Detail, Metric, UnavailableState, WarningList } from "./ui";
import { makeEntry } from "../test/fixtures";
import type { DatabaseStatus } from "../types";

const status: DatabaseStatus = {
  dbPath: "C:\\Users\\jtill\\.capsule\\capsule.db",
  dbExists: true,
  dbSizeBytes: 1024,
  dbModifiedAt: "2026-07-04T12:00:00Z",
  readable: false,
  schemaSummary: {
    tableCount: 0,
    detectedTables: [],
    hasEntriesTable: false,
    hasTagsTable: false,
    hasFtsTable: false,
    missingCoreTables: ["entries"],
  },
  entryCount: null,
  tagCount: null,
  backupCount: null,
  lastBackupPath: null,
  security: {
    mode: "unknown",
    locked: true,
    readable: false,
    message: "Database is locked.",
  },
  warnings: [],
};

describe("shared UI components", () => {
  test("renders detail and metric label/value pairs", () => {
    render(
      <>
        <dl>
          <Detail label="Number" value="#42" />
        </dl>
        <Metric label="Entries" value={makeEntry().id} />
      </>,
    );

    expect(screen.getByText("Number")).toBeInTheDocument();
    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.getByText("Entries")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();
  });

  test("renders warning lists only when warnings exist", () => {
    const { container, rerender } = render(<WarningList warnings={[]} />);
    expect(container).toBeEmptyDOMElement();

    rerender(<WarningList warnings={["First warning", "Second warning"]} />);

    expect(screen.getByRole("list")).toBeInTheDocument();
    expect(screen.getByText("First warning")).toBeInTheDocument();
    expect(screen.getByText("Second warning")).toBeInTheDocument();
  });

  test("shows unavailable state context with the active database path", () => {
    render(<UnavailableState icon={<span aria-hidden="true">!</span>} label="Needs database" status={status} />);

    expect(screen.getByRole("heading", { name: "Needs database" })).toBeInTheDocument();
    expect(screen.getByText(status.dbPath)).toBeInTheDocument();
  });
});
