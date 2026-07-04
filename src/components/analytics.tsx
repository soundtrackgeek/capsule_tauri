import type { AnalyticsResponse } from "../types";
import { formatMoodSentiment, sentimentPosition } from "../lib/analytics";

export function TrendBars({ trend }: { trend: AnalyticsResponse["monthlyTrend"] }) {
  if (trend.length === 0) {
    return <p className="muted">No monthly activity in this period.</p>;
  }

  const maxEntries = Math.max(1, ...trend.map((point) => point.entryCount));
  return (
    <div className="bar-list">
      {trend.map((point) => (
        <div className="bar-row" key={point.period}>
          <span>{point.period}</span>
          <div className="bar-track">
            <div style={{ width: `${(point.entryCount / maxEntries) * 100}%` }} />
          </div>
          <strong>{point.entryCount}</strong>
          <em>{point.wordCount} words</em>
        </div>
      ))}
    </div>
  );
}

export function MoodTrendBars({ trend }: { trend: AnalyticsResponse["monthlyTrend"] }) {
  if (trend.length === 0) {
    return <p className="muted">No monthly activity in this period.</p>;
  }

  if (!trend.some((point) => point.averageMoodSentiment !== null)) {
    return <p className="muted">No rated moods in this period.</p>;
  }

  return (
    <div className="bar-list">
      {trend.map((point) => (
        <div className="bar-row bar-row--sentiment" key={point.period}>
          <span>{point.period}</span>
          <div className="sentiment-track">
            {point.averageMoodSentiment !== null && (
              <i
                aria-hidden="true"
                className="sentiment-marker"
                style={{ left: `${sentimentPosition(point.averageMoodSentiment)}%` }}
              />
            )}
          </div>
          <strong>{formatMoodSentiment(point.averageMoodSentiment)}</strong>
          <em>{point.moodSentimentCount} moods</em>
        </div>
      ))}
    </div>
  );
}

export function BreakdownList({
  items,
  emptyText = "No data in this period.",
}: {
  items: AnalyticsResponse["tagBreakdown"];
  emptyText?: string;
}) {
  if (items.length === 0) {
    return <p className="muted">{emptyText}</p>;
  }

  const maxCount = Math.max(1, ...items.map((item) => item.count));
  return (
    <div className="bar-list">
      {items.slice(0, 10).map((item) => (
        <div className="bar-row" key={item.label}>
          <span>{item.label}</span>
          <div className="bar-track">
            <div style={{ width: `${(item.count / maxCount) * 100}%` }} />
          </div>
          <strong>{item.count}</strong>
        </div>
      ))}
    </div>
  );
}
