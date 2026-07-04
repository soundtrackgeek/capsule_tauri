import type { Entry } from "../types";

export function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: 42,
    uuid: "entry_test42",
    createdAt: "2026-07-04 12:00",
    updatedAt: "2026-07-04 12:05",
    text: "A focused test entry with enough body text to render useful previews.",
    textPlain: "A focused test entry with enough body text to render useful previews.",
    contentFormat: "markdown",
    title: "Test entry",
    summary: "A compact summary for the test entry.",
    mood: "focused",
    moodInfo: { name: "focused", label: "Focused" },
    tags: [
      { id: 1, name: "work" },
      { id: 2, name: "capsule" },
    ],
    starred: false,
    pinned: false,
    hidden: false,
    location: {
      latitude: 69.65,
      longitude: 18.96,
      placeName: "Tromso, Norway",
      source: "manual",
      weatherCondition: "Clear",
      weatherTempC: 8,
      weatherTempF: null,
      weatherIcon: "clear",
      weatherHumidity: 70,
      weatherWindKph: 9.5,
      weatherFetchedAt: "2026-07-04 12:01",
    },
    thread: null,
    attachmentCount: 2,
    ...overrides,
  };
}
