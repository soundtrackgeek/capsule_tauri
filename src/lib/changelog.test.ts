import { describe, expect, test } from "vitest";

import { parseChangelog } from "./changelog";

describe("parseChangelog", () => {
  test("groups versions, categories, and wrapped bullet entries", () => {
    const markdown = `# Changelog

## 0.20.0 - 2026-07-03

### Added

- Added an About changelog panel backed by the project changelog.
- Added a wrapped entry that continues
  on the next line.

### Fixed

- Fixed a display issue.

## 0.19.2 - 2026-07-03

### Changed

- Bumped the app version to 0.19.2.
`;

    expect(parseChangelog(markdown)).toEqual([
      {
        version: "0.20.0",
        date: "2026-07-03",
        sections: [
          {
            title: "Added",
            items: [
              "Added an About changelog panel backed by the project changelog.",
              "Added a wrapped entry that continues on the next line.",
            ],
          },
          {
            title: "Fixed",
            items: ["Fixed a display issue."],
          },
        ],
      },
      {
        version: "0.19.2",
        date: "2026-07-03",
        sections: [
          {
            title: "Changed",
            items: ["Bumped the app version to 0.19.2."],
          },
        ],
      },
    ]);
  });
});
