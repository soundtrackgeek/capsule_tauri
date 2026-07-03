import { describe, expect, test } from "vitest";

import { formatEntryNumber } from "./format";

describe("formatEntryNumber", () => {
  test("formats positive entry IDs as old Capsule numbers", () => {
    expect(formatEntryNumber(42)).toBe("#42");
  });

  test("falls back when an entry ID is unavailable", () => {
    expect(formatEntryNumber(0)).toBe("#?");
    expect(formatEntryNumber(null)).toBe("#?");
    expect(formatEntryNumber(undefined)).toBe("#?");
  });
});
