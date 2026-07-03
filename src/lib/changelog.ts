export type ChangelogSection = {
  title: string;
  items: string[];
};

export type ChangelogRelease = {
  version: string;
  date: string | null;
  sections: ChangelogSection[];
};

const releaseHeadingPattern =
  /^(?:\[(?<bracketedVersion>[^\]]+)\]|(?<version>.+?))(?:\s+-\s+(?<date>\d{4}-\d{2}-\d{2}))?$/;

export function parseChangelog(markdown: string): ChangelogRelease[] {
  const releases: ChangelogRelease[] = [];
  let currentRelease: ChangelogRelease | null = null;
  let currentSection: ChangelogSection | null = null;

  const pushSection = () => {
    if (currentRelease && currentSection && currentSection.items.length > 0) {
      currentRelease.sections.push(currentSection);
    }
    currentSection = null;
  };

  const pushRelease = () => {
    pushSection();
    if (currentRelease && currentRelease.sections.length > 0) {
      releases.push(currentRelease);
    }
    currentRelease = null;
  };

  for (const line of markdown.split(/\r?\n/)) {
    if (line.startsWith("## ")) {
      pushRelease();
      const heading = line.replace(/^##\s+/, "").trim();
      const match = heading.match(releaseHeadingPattern);
      currentRelease = {
        version: (match?.groups?.bracketedVersion ?? match?.groups?.version ?? heading).trim(),
        date: match?.groups?.date ?? null,
        sections: [],
      };
      continue;
    }

    if (line.startsWith("### ")) {
      pushSection();
      if (currentRelease) {
        currentSection = { title: line.replace(/^###\s+/, "").trim(), items: [] };
      }
      continue;
    }

    if (!currentSection) {
      continue;
    }

    if (line.startsWith("- ")) {
      currentSection.items.push(line.replace(/^-\s+/, "").trim());
      continue;
    }

    const continuation = line.match(/^\s{2,}(.+)$/);
    const lastItemIndex = currentSection.items.length - 1;
    if (continuation && lastItemIndex >= 0) {
      currentSection.items[lastItemIndex] = `${currentSection.items[lastItemIndex]} ${continuation[1].trim()}`;
    }
  }

  pushRelease();
  return releases;
}
