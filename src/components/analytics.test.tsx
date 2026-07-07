import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { ActivityTrends } from "./analytics";
import type { AnalyticsResponse } from "../types";

function makeAnalytics(): AnalyticsResponse {
  return {
    overview: {
      totalEntries: 3,
      totalWords: 120,
      averageWords: 40,
      averageMoodSentiment: null,
      moodSentimentCount: 0,
      totalImages: 0,
      entriesWithImages: 0,
      entriesWithLocation: 0,
      longestStreakDays: 2,
      currentStreakDays: 2,
    },
    monthlyTrend: [],
    dailyTrend: [
      { date: "2026-07-01", entryCount: 2, wordCount: 50 },
      { date: "2026-07-02", entryCount: 1, wordCount: 70 },
    ],
    hourlyTrend: Array.from({ length: 24 }, (_, hour) => ({
      hour,
      label: `${String(hour).padStart(2, "0")}:00`,
      entryCount: hour === 9 ? 1 : 0,
      wordCount: hour === 9 ? 70 : 0,
    })),
    weekdayTrend: [
      { dayNum: 1, label: "Monday", shortLabel: "Mon", entryCount: 0, wordCount: 0 },
      { dayNum: 2, label: "Tuesday", shortLabel: "Tue", entryCount: 0, wordCount: 0 },
      { dayNum: 3, label: "Wednesday", shortLabel: "Wed", entryCount: 1, wordCount: 50 },
      { dayNum: 4, label: "Thursday", shortLabel: "Thu", entryCount: 1, wordCount: 70 },
      { dayNum: 5, label: "Friday", shortLabel: "Fri", entryCount: 0, wordCount: 0 },
      { dayNum: 6, label: "Saturday", shortLabel: "Sat", entryCount: 0, wordCount: 0 },
      { dayNum: 0, label: "Sunday", shortLabel: "Sun", entryCount: 0, wordCount: 0 },
    ],
    writingWindow: {
      days: [],
      summary: {
        activeDays: 0,
        totalEntries: 0,
        avgFirstTime: null,
        avgLastTime: null,
        avgSpanMinutes: 0,
        earliestFirstTime: null,
        latestLastTime: null,
        longestSpanDay: null,
      },
    },
    locationActivity: [],
    moodBreakdown: [],
    tagBreakdown: [],
    locationBreakdown: [],
    weatherBreakdown: [],
    topWords: [],
    warnings: [],
  };
}

describe("ActivityTrends", () => {
  test("shows mouse-follow tooltips for bars and line points", () => {
    const { container } = render(<ActivityTrends analytics={makeAnalytics()} />);

    const barHitTarget = container.querySelector(".activity-hit-target");
    expect(barHitTarget).toBeInTheDocument();
    fireEvent.mouseMove(barHitTarget!, { clientX: 120, clientY: 180 });

    expect(screen.getByRole("tooltip")).toHaveTextContent("2026-07-01");
    expect(screen.getByRole("tooltip")).toHaveTextContent("2 entries");
    expect(screen.getByRole("tooltip")).toHaveTextContent("50 words");

    fireEvent.click(screen.getByRole("tab", { name: "Words Over Time" }));
    const lineHitTarget = container.querySelector(".activity-hit-target");
    fireEvent.mouseMove(lineHitTarget!, { clientX: 140, clientY: 190 });

    expect(screen.getByRole("tooltip")).toHaveTextContent("2026-07-01");
    expect(screen.getByRole("tooltip")).toHaveTextContent("words: 50");
  });
});
