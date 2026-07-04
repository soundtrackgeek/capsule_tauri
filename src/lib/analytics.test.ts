import { describe, expect, test } from "vitest";
import { formatMoodSentiment, sentimentPosition } from "./analytics";

describe("analytics formatting helpers", () => {
  test("formats missing and signed mood sentiment values", () => {
    expect(formatMoodSentiment(null)).toBe("n/a");
    expect(formatMoodSentiment(undefined)).toBe("n/a");
    expect(formatMoodSentiment(0.235)).toBe("+0.23");
    expect(formatMoodSentiment(-0.4)).toBe("-0.40");
    expect(formatMoodSentiment(0.001)).toBe("0.00");
  });

  test("maps sentiment values into a bounded percentage position", () => {
    expect(sentimentPosition(-1)).toBe(0);
    expect(sentimentPosition(0)).toBe(50);
    expect(sentimentPosition(1)).toBe(100);
    expect(sentimentPosition(2)).toBe(100);
  });
});
