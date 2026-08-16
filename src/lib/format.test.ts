import { describe, expect, test } from "vitest";

import { countWords, formatEntryNumber, formatWordCount } from "./format";

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

describe("word counts", () => {
  test("counts words using the same whitespace boundaries as journal analytics", () => {
    expect(countWords("First line.\n\nSecond\tline.")).toBe(4);
    expect(countWords("   ")).toBe(0);
  });

  test("formats singular and plural word counts", () => {
    expect(formatWordCount(1)).toBe("1 word");
    expect(formatWordCount(12)).toBe("12 words");
  });
});
