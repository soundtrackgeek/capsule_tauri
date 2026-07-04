import { describe, expect, test } from "vitest";
import {
  buildCalendarMonths,
  calendarDayTitle,
  calendarLevel,
  calendarSentimentClass,
} from "./calendar";
import type { WritingCalendarDay } from "../types";

const activeDay: WritingCalendarDay = {
  date: "2026-07-04",
  entryCount: 2,
  wordCount: 320,
  imageCount: 1,
  moods: ["happy"],
  averageMoodSentiment: 0.67,
  moodSentimentCount: 1,
};

describe("calendar helpers", () => {
  test("builds months with active day data attached", () => {
    const months = buildCalendarMonths(2026, [activeDay]);

    expect(months).toHaveLength(12);
    expect(months[6].label).toBe("July");
    expect(months[6].days.find((day) => day.date === activeDay.date)?.data).toEqual(activeDay);
  });

  test("formats activity level, sentiment class, and title text", () => {
    expect(calendarLevel(activeDay, 4)).toBe(2);
    expect(calendarSentimentClass(activeDay)).toBe("calendar-day--sentiment-positive");
    expect(calendarDayTitle(activeDay.date, activeDay)).toBe(
      "2026-07-04: 2 entries, 320 words, 1 images / happy / mood +0.67",
    );
  });
});
