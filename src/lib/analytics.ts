export function formatMoodSentiment(value: number | null | undefined) {
  if (value === null || value === undefined) {
    return "n/a";
  }

  const rounded = Math.abs(value) < 0.005 ? 0 : value;
  return `${rounded > 0 ? "+" : ""}${rounded.toFixed(2)}`;
}

export function sentimentPosition(value: number) {
  return Math.max(0, Math.min(100, ((value + 1) / 2) * 100));
}
