import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { ActivityTrends, MoodTimingAnalytics, WeatherAnalytics } from "./analytics";
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
    captureSources: {
      totalEntries: 3,
      mobileEntries: 1,
      desktopEntries: 2,
      mobilePercent: 33.333,
      desktopPercent: 66.667,
      trend: [
        { period: "2026-07-01", mobileCount: 1, desktopCount: 1, totalCount: 2, mobilePercent: 50 },
      ],
    },
    moodTrend: [{ date: "2026-07-01", averageSentiment: 0.4, moodCount: 2 }],
    moodTiming: {
      timeOfDay: [
        { key: "morning", label: "Morning", detail: "06:00-10:00", averageSentiment: 0.4, moodCount: 2, topMood: "focused" },
      ],
      dayOfWeek: [
        { key: "3", label: "Wed", detail: "Wednesday", averageSentiment: 0.4, moodCount: 2, topMood: "focused" },
      ],
    },
    weather: {
      overview: {
        totalEntries: 3,
        entriesWithWeather: 2,
        coveragePercent: 66.667,
        uniqueConditions: 1,
        averageTempC: 8,
        minTempC: 7,
        maxTempC: 9,
        mostCommonCondition: "Clear",
      },
      temperatureBuckets: [{ key: "5-10", label: "5-10 C", count: 2 }],
      trend: [{ date: "2026-07-01", averageTempC: 8, averageHumidity: 70, averageWindKph: 9, entryCount: 2 }],
      conditionMood: [{ condition: "Clear", averageSentiment: 0.4, moodCount: 2, topMood: "focused" }],
      temperatureMoodCorrelation: 0.2,
      conditionTags: [{ condition: "Clear", entryCount: 2, topTags: [{ label: "work", count: 1, percent: 50 }] }],
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

  test("shows mood sample counts and switches weather metrics", () => {
    const analytics = makeAnalytics();
    render(
      <>
        <MoodTimingAnalytics analytics={analytics} />
        <WeatherAnalytics analytics={analytics} />
      </>,
    );

    expect(screen.getByLabelText("Morning: +0.40, 2 rated moods")).toBeInTheDocument();
    expect(screen.getByText(/Associations are descriptive and do not establish causation/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("tab", { name: "Humidity" }));
    expect(screen.getByRole("img", { name: "humidity chart" })).toBeInTheDocument();
  });
});
