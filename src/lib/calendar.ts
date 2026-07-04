import type { WritingCalendarResponse } from "../types";
import { formatMoodSentiment } from "./analytics";

export type CalendarDayCell = {
  date: string;
  day: number;
  data?: WritingCalendarResponse["days"][number];
};

export type CalendarMonth = {
  label: string;
  blanks: number;
  days: CalendarDayCell[];
};

export function buildCalendarMonths(
  year: number,
  days: WritingCalendarResponse["days"],
): CalendarMonth[] {
  const daysByDate = new Map(days.map((day) => [day.date, day]));
  return Array.from({ length: 12 }).map((_, monthIndex) => {
    const firstDay = new Date(year, monthIndex, 1).getDay();
    const daysInMonth = new Date(year, monthIndex + 1, 0).getDate();
    const label = new Date(year, monthIndex, 1).toLocaleDateString(undefined, {
      month: "long",
    });
    return {
      label,
      blanks: firstDay,
      days: Array.from({ length: daysInMonth }).map((__, dayIndex) => {
        const day = dayIndex + 1;
        const date = `${year}-${String(monthIndex + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
        return {
          date,
          day,
          data: daysByDate.get(date),
        };
      }),
    };
  });
}

export function calendarLevel(day: CalendarDayCell["data"], maxEntryCount: number) {
  if (!day || day.entryCount <= 0) {
    return 0;
  }
  return Math.max(1, Math.ceil((day.entryCount / Math.max(maxEntryCount, 1)) * 4));
}

export function calendarSentimentClass(day: CalendarDayCell["data"]) {
  const value = day?.averageMoodSentiment;
  if (value === null || value === undefined) {
    return "";
  }

  if (value >= 0.2) {
    return "calendar-day--sentiment-positive";
  }

  if (value <= -0.2) {
    return "calendar-day--sentiment-negative";
  }

  return "calendar-day--sentiment-neutral";
}

export function calendarDayTitle(date: string, day: CalendarDayCell["data"]) {
  if (!day) {
    return `${date}: no entries`;
  }
  const moodText = day.moods.length ? ` / ${day.moods.join(", ")}` : "";
  const sentimentText =
    day.averageMoodSentiment !== null
      ? ` / mood ${formatMoodSentiment(day.averageMoodSentiment)}`
      : "";
  return `${date}: ${day.entryCount} entries, ${day.wordCount} words, ${day.imageCount} images${moodText}${sentimentText}`;
}
