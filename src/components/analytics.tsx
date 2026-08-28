import { useMemo, useState, type MouseEvent as ReactMouseEvent, type ReactNode } from "react";
import type { AnalyticsResponse } from "../types";
import { formatMoodSentiment, sentimentPosition } from "../lib/analytics";

type ActivityTrendMode =
  | "entries"
  | "words"
  | "writingWindow"
  | "wordsHourly"
  | "wordsWeekday"
  | "notesHourly"
  | "notesWeekday"
  | "location";

type BarPoint = {
  key: string;
  label: string;
  value: number;
  detail?: string;
};

type LinePoint = {
  key: string;
  label: string;
  value: number | null;
  detail?: string;
};

type LineSeries = {
  key: string;
  label: string;
  color: string;
  points: LinePoint[];
};

type ActivityTooltipState = {
  color: string;
  detail?: string;
  title: string;
  value: string;
  x: number;
  y: number;
};

const ACTIVITY_TABS: Array<{ id: ActivityTrendMode; label: string }> = [
  { id: "entries", label: "Entry Frequency" },
  { id: "words", label: "Words Over Time" },
  { id: "writingWindow", label: "First / Last Capsule" },
  { id: "wordsHourly", label: "Words per Hour" },
  { id: "wordsWeekday", label: "Words per Day" },
  { id: "notesHourly", label: "Notes per Hour" },
  { id: "notesWeekday", label: "Notes per Day" },
  { id: "location", label: "Entries per Location" },
];

const CHART_WIDTH = 960;
const CHART_HEIGHT = 300;
const BAR_MARGINS = { top: 16, right: 18, bottom: 52, left: 54 };
const LINE_MARGINS = { top: 16, right: 22, bottom: 52, left: 58 };

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

export function CaptureSourceAnalytics({ analytics }: { analytics: AnalyticsResponse }) {
  const sources = analytics.captureSources;
  const trend = sources.trend.slice(-30).map((point) => ({
    key: point.period,
    label: point.period,
    value: point.mobilePercent,
    detail: `${point.mobileCount} mobile and ${point.desktopCount} desktop entries`,
  }));

  return (
    <div className="analytics-feature-stack">
      <p className="analytics-caption">
        Desktop includes every entry without mobile provenance. Mobile requires an attached location row whose source is marked mobile.
      </p>
      <div className="analytics-summary-grid analytics-summary-grid--three">
        <AnalyticsSummary label="Mobile share" value={`${sources.mobilePercent.toFixed(1)}%`} />
        <AnalyticsSummary label="Desktop" value={formatNumber(sources.desktopEntries)} detail={`${sources.desktopPercent.toFixed(1)}%`} />
        <AnalyticsSummary label="Mobile" value={formatNumber(sources.mobileEntries)} detail={`${sources.mobilePercent.toFixed(1)}%`} />
      </div>
      <div
        aria-label={`${sources.desktopPercent.toFixed(1)} percent desktop, ${sources.mobilePercent.toFixed(1)} percent mobile`}
        className="source-share"
        role="img"
      >
        <span className="source-share__desktop" style={{ width: `${sources.desktopPercent}%` }} />
        <span className="source-share__mobile" style={{ width: `${sources.mobilePercent}%` }} />
      </div>
      {sources.totalEntries > 0 && (
        <SvgLineChart
          color="#0ea5e9"
          emptyText="No capture-source history in this period."
          maxValueOverride={100}
          points={trend}
          valueFormatter={(value) => `${value.toFixed(1)}%`}
          valueLabel="mobile share"
        />
      )}
    </div>
  );
}

export function MoodSentimentChart({ analytics }: { analytics: AnalyticsResponse }) {
  const points = analytics.moodTrend.map((point) => ({
    key: point.date,
    label: point.date,
    value: point.averageSentiment,
    detail: `${point.moodCount} rated mood${point.moodCount === 1 ? "" : "s"}`,
  }));
  return (
    <div className="analytics-feature-stack">
      <p className="analytics-caption">Average rated mood per active day, from -1.0 to +1.0.</p>
      <SentimentLineChart emptyText="No rated moods in this period." points={points} />
    </div>
  );
}

export function MoodTimingAnalytics({ analytics }: { analytics: AnalyticsResponse }) {
  return (
    <div className="analytics-feature-grid analytics-feature-grid--two mood-timing-grid">
      <section className="analytics-subsection" aria-labelledby="mood-time-heading">
        <h4 id="mood-time-heading">Mood by Time of Day</h4>
        <p className="analytics-caption">Average sentiment and sample count per time phase.</p>
        <SignedBarChart emptyText="No rated moods in this period." points={analytics.moodTiming.timeOfDay} />
      </section>
      <section className="analytics-subsection" aria-labelledby="mood-weekday-heading">
        <h4 id="mood-weekday-heading">Mood by Day of Week</h4>
        <p className="analytics-caption">Average sentiment and sample count per weekday.</p>
        <SignedBarChart emptyText="No rated moods in this period." points={analytics.moodTiming.dayOfWeek} />
      </section>
    </div>
  );
}

export function MoodDistribution({ analytics }: { analytics: AnalyticsResponse }) {
  const items = analytics.moodBreakdown;
  const total = items.reduce((sum, item) => sum + item.count, 0);
  if (items.length === 0) return <p className="muted">No moods in this period.</p>;
  return (
    <div className="distribution-list" aria-label={`Mood distribution across ${total} entries`} role="list">
      {items.map((item, index) => {
        const percentValue = total > 0 ? item.count * 100 / total : 0;
        return (
          <div className="distribution-row" key={item.label} role="listitem">
            <span aria-hidden="true" className={`distribution-swatch distribution-swatch--${index % 6}`} />
            <strong>{item.label}</strong>
            <div className="distribution-track" aria-hidden="true">
              <span className={`distribution-fill distribution-fill--${index % 6}`} style={{ width: `${percentValue}%` }} />
            </div>
            <span>{item.count}</span>
            <em>{percentValue.toFixed(0)}%</em>
          </div>
        );
      })}
    </div>
  );
}

type WeatherMetric = "temperature" | "humidity" | "wind";
type WeatherGroup = "day" | "week" | "month";

export function WeatherAnalytics({ analytics }: { analytics: AnalyticsResponse }) {
  const [metric, setMetric] = useState<WeatherMetric>("temperature");
  const [group, setGroup] = useState<WeatherGroup>("day");
  const weather = analytics.weather;
  const overview = weather.overview;
  const trendPoints = groupWeatherTrend(weather.trend, metric, group).map((point) => {
    const value = point.value;
    const unit = metric === "temperature" ? "C" : metric === "humidity" ? "%" : "km/h";
    return {
      key: point.date,
      label: point.date,
      value,
      detail: `${point.entryCount} weather sample${point.entryCount === 1 ? "" : "s"}; ${value === null ? "no value" : `${value.toFixed(1)} ${unit}`}`,
    };
  });
  const correlation = weather.temperatureMoodCorrelation;
  const bestCondition = weather.conditionMood[0];
  const weatherMoodHeadline = bestCondition
    ? `Rated moods average highest on ${bestCondition.condition.toLowerCase()} days.`
    : "Not enough rated moods to compare weather conditions.";
  const correlationText = correlation === null
    ? "Temperature correlation needs at least three varied temperature and mood samples."
    : `Temperature and sentiment correlation: r = ${correlation.toFixed(2)} (${correlationStrength(correlation)}).`;
  const tagHeadline = weather.conditionTags[0]?.topTags[0]
    ? `Your most frequent weather-linked tag is “${weather.conditionTags[0].topTags[0].label}” on ${weather.conditionTags[0].condition.toLowerCase()} days.`
    : "Add tags to weather-linked entries to reveal activity patterns.";

  if (overview.entriesWithWeather === 0) {
    return <p className="muted">No weather metadata in this period.</p>;
  }

  return (
    <div className="analytics-feature-stack">
      <p className="analytics-caption">
        Weather coverage and patterns for the selected period. Associations are descriptive and do not establish causation.
      </p>
      <div className="analytics-summary-grid analytics-summary-grid--four">
        <AnalyticsSummary label="Entries with weather" value={formatNumber(overview.entriesWithWeather)} detail={`${overview.coveragePercent.toFixed(0)}% coverage`} />
        <AnalyticsSummary label="Average temperature" value={formatTemperature(overview.averageTempC)} />
        <AnalyticsSummary label="Temperature range" value={formatTemperatureRange(overview.minTempC, overview.maxTempC)} detail={`${overview.uniqueConditions} conditions`} />
        <AnalyticsSummary label="Most common" value={overview.mostCommonCondition ?? "No data"} />
      </div>

      <div className="analytics-feature-grid analytics-feature-grid--two">
        <section className="analytics-subsection" aria-labelledby="weather-condition-heading">
          <h4 id="weather-condition-heading">Weather Condition Distribution</h4>
          <MiniBarList items={analytics.weatherBreakdown} />
        </section>
        <section className="analytics-subsection" aria-labelledby="temperature-distribution-heading">
          <h4 id="temperature-distribution-heading">Temperature Distribution</h4>
          <MiniBarList items={weather.temperatureBuckets.map((bucket) => ({ label: bucket.label, count: bucket.count }))} />
        </section>
      </div>

      <section className="analytics-subsection" aria-labelledby="weather-trends-heading">
        <div className="analytics-section-heading">
          <div>
            <h4 id="weather-trends-heading">Weather Trends</h4>
            <p className="analytics-caption">Averages for entries with captured weather.</p>
          </div>
          <div className="activity-tabs" role="tablist" aria-label="Weather trend metric">
            {(["temperature", "humidity", "wind"] as WeatherMetric[]).map((item) => (
              <button
                aria-selected={metric === item}
                className={metric === item ? "activity-tab activity-tab--active" : "activity-tab"}
                key={item}
                onClick={() => setMetric(item)}
                role="tab"
                type="button"
              >
                {item === "temperature" ? "Temperature" : item === "humidity" ? "Humidity" : "Wind"}
              </button>
            ))}
          </div>
          <div className="activity-tabs" role="tablist" aria-label="Weather trend grouping">
            {(["day", "week", "month"] as WeatherGroup[]).map((item) => (
              <button
                aria-selected={group === item}
                className={group === item ? "activity-tab activity-tab--active" : "activity-tab"}
                key={item}
                onClick={() => setGroup(item)}
                role="tab"
                type="button"
              >
                {item[0].toUpperCase() + item.slice(1)}
              </button>
            ))}
          </div>
        </div>
        <SvgLineChart
          color={metric === "temperature" ? "#f97316" : metric === "humidity" ? "#0ea5e9" : "#14b8a6"}
          emptyText={`No ${metric} data in this period.`}
          points={trendPoints}
          valueFormatter={(value) => `${value.toFixed(1)}${metric === "temperature" ? " C" : metric === "humidity" ? "%" : " km/h"}`}
          valueLabel={metric}
        />
      </section>

      <section className="analytics-subsection" aria-labelledby="weather-mood-heading">
        <h4 id="weather-mood-heading">Weather vs Mood Sentiment</h4>
        <div className="analytics-insight">
          <strong>{weatherMoodHeadline}</strong>
          <span>{correlationText}</span>
        </div>
        <SignedBarChart
          emptyText="No weather conditions have rated mood samples."
          points={weather.conditionMood.map((point) => ({
            key: point.condition,
            label: point.condition,
            detail: point.topMood ? `Top mood: ${point.topMood}` : "Rated moods",
            averageSentiment: point.averageSentiment,
            moodCount: point.moodCount,
            topMood: point.topMood,
          }))}
        />
      </section>

      <section className="analytics-subsection" aria-labelledby="weather-tags-heading">
        <h4 id="weather-tags-heading">What You Do in Different Weather</h4>
        <p className="analytics-caption">{tagHeadline}</p>
        <div className="weather-tag-grid">
          {weather.conditionTags.map((condition) => (
            <article className="weather-tag-group" key={condition.condition}>
              <h5>{condition.condition} <span>({condition.entryCount})</span></h5>
              {condition.topTags.map((tag) => (
                <div key={tag.label}>
                  <span>{tag.label}</span>
                  <strong>{tag.percent.toFixed(0)}%</strong>
                </div>
              ))}
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

function AnalyticsSummary({ label, value, detail }: { label: string; value: ReactNode; detail?: ReactNode }) {
  return (
    <div className="analytics-summary">
      <span>{label}</span>
      <strong>{value}</strong>
      {detail && <em>{detail}</em>}
    </div>
  );
}

function MiniBarList({ items }: { items: Array<{ label: string; count: number }> }) {
  const max = Math.max(1, ...items.map((item) => item.count));
  if (!items.some((item) => item.count > 0)) return <p className="muted">No data in this period.</p>;
  return (
    <div className="mini-bar-list">
      {items.map((item) => (
        <div className="mini-bar-row" key={item.label}>
          <span>{item.label}</span>
          <div aria-hidden="true"><i style={{ width: `${item.count * 100 / max}%` }} /></div>
          <strong>{item.count}</strong>
        </div>
      ))}
    </div>
  );
}

function SignedBarChart({
  points,
  emptyText,
}: {
  points: AnalyticsResponse["moodTiming"]["timeOfDay"];
  emptyText: string;
}) {
  const [tooltip, setTooltip] = useState<ActivityTooltipState | null>(null);
  const populated = points.filter((point) => point.averageSentiment !== null && point.moodCount > 0);
  if (populated.length === 0) return <p className="muted activity-empty">{emptyText}</p>;
  const width = 720;
  const height = 270;
  const margins = { top: 18, right: 18, bottom: 62, left: 44 };
  const innerWidth = width - margins.left - margins.right;
  const innerHeight = height - margins.top - margins.bottom;
  const zeroY = margins.top + innerHeight / 2;
  const band = innerWidth / points.length;
  const barWidth = Math.min(64, band * 0.62);
  const yFor = (value: number) => zeroY - value * innerHeight / 2;
  return (
    <>
      <div className="activity-chart-wrap signed-chart-desktop" onMouseLeave={() => setTooltip(null)}>
        <svg aria-label="Average mood sentiment chart" className="activity-chart" role="img" viewBox={`0 0 ${width} ${height}`}>
        {[-1, -0.5, 0, 0.5, 1].map((tick) => (
          <g key={tick}>
            <line className={tick === 0 ? "activity-axis-line" : "activity-grid-line"} x1={margins.left} x2={margins.left + innerWidth} y1={yFor(tick)} y2={yFor(tick)} />
            <text className="activity-axis-label" textAnchor="end" x={margins.left - 7} y={yFor(tick) + 4}>{tick > 0 ? `+${tick}` : tick}</text>
          </g>
        ))}
        {points.map((point, index) => {
          const value = point.averageSentiment;
          const x = margins.left + index * band + (band - barWidth) / 2;
          const y = value === null ? zeroY : Math.min(zeroY, yFor(value));
          const barHeight = value === null ? 0 : Math.abs(yFor(value) - zeroY);
          const color = value === null ? "#94a3b8" : sentimentColor(value);
          return (
            <g key={point.key}>
              <rect className="activity-bar" fill={color} height={barHeight} rx="4" width={barWidth} x={x} y={y} />
              <rect
                aria-label={`${point.label}: ${formatMoodSentiment(value)}, ${point.moodCount} rated moods`}
                className="activity-hit-target"
                height={innerHeight}
                onMouseEnter={(event) => setTooltip({ ...tooltipPosition(event), color, detail: `${point.detail}; ${point.topMood ? `top mood ${point.topMood}; ` : ""}${point.moodCount} rated moods`, title: point.label, value: formatMoodSentiment(value) })}
                onMouseMove={(event) => setTooltip({ ...tooltipPosition(event), color, detail: `${point.detail}; ${point.topMood ? `top mood ${point.topMood}; ` : ""}${point.moodCount} rated moods`, title: point.label, value: formatMoodSentiment(value) })}
                width={band}
                x={margins.left + index * band}
                y={margins.top}
              />
              <text className="activity-axis-label" textAnchor="middle" x={x + barWidth / 2} y={height - 35}>{truncateLabel(point.label, 14)}</text>
              <text className="activity-axis-label" textAnchor="middle" x={x + barWidth / 2} y={height - 18}>n={point.moodCount}</text>
            </g>
          );
        })}
        </svg>
        <ActivityTooltip tooltip={tooltip} />
      </div>
      <div className="signed-chart-mobile" aria-label="Average mood sentiment values" role="list">
        {points.map((point) => {
          const value = point.averageSentiment;
          const position = value === null ? 50 : sentimentPosition(value);
          return (
            <div className="signed-mobile-row" key={point.key} role="listitem">
              <div>
                <strong>{point.label}</strong>
                <span>{point.detail}</span>
              </div>
              <div className="signed-mobile-track" aria-hidden="true">
                <i className="signed-mobile-zero" />
                {value !== null && <i className="signed-mobile-marker" style={{ background: sentimentColor(value), left: `${position}%` }} />}
              </div>
              <div>
                <strong>{formatMoodSentiment(value)}</strong>
                <span>n={point.moodCount}</span>
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}

function SentimentLineChart({ points, emptyText }: { points: LinePoint[]; emptyText: string }) {
  const [tooltip, setTooltip] = useState<ActivityTooltipState | null>(null);
  if (points.length === 0) return <p className="muted activity-empty">{emptyText}</p>;
  const width = CHART_WIDTH;
  const height = CHART_HEIGHT;
  const innerWidth = width - LINE_MARGINS.left - LINE_MARGINS.right;
  const innerHeight = height - LINE_MARGINS.top - LINE_MARGINS.bottom;
  const xFor = (index: number) => points.length <= 1 ? LINE_MARGINS.left + innerWidth / 2 : LINE_MARGINS.left + index * innerWidth / (points.length - 1);
  const yFor = (value: number) => LINE_MARGINS.top + (1 - (value + 1) / 2) * innerHeight;
  const path = points.map((point, index) => `${index === 0 ? "M" : "L"} ${xFor(index)} ${yFor(point.value ?? 0)}`).join(" ");
  const labelStep = dailyLabelStep(points.length);
  return (
    <div className="activity-chart-wrap" onMouseLeave={() => setTooltip(null)}>
      <svg aria-label="Mood sentiment over time" className="activity-chart" role="img" viewBox={`0 0 ${width} ${height}`}>
        {[-1, -0.5, 0, 0.5, 1].map((tick) => (
          <g key={tick}>
            <line className={tick === 0 ? "activity-axis-line" : "activity-grid-line"} x1={LINE_MARGINS.left} x2={LINE_MARGINS.left + innerWidth} y1={yFor(tick)} y2={yFor(tick)} />
            <text className="activity-axis-label" textAnchor="end" x={LINE_MARGINS.left - 8} y={yFor(tick) + 4}>{tick > 0 ? `+${tick}` : tick}</text>
          </g>
        ))}
        <path className="activity-line" d={path} stroke="#8b5cf6" />
        {points.map((point, index) => {
          const x = xFor(index);
          const y = yFor(point.value ?? 0);
          return (
            <g key={point.key}>
              <circle className="activity-dot" cx={x} cy={y} fill="#8b5cf6" r="5" />
              <circle
                aria-label={`${point.label}: ${formatMoodSentiment(point.value)}`}
                className="activity-hit-target"
                cx={x}
                cy={y}
                onMouseEnter={(event) => setTooltip({ ...tooltipPosition(event), color: "#8b5cf6", detail: point.detail, title: point.label, value: formatMoodSentiment(point.value) })}
                onMouseMove={(event) => setTooltip({ ...tooltipPosition(event), color: "#8b5cf6", detail: point.detail, title: point.label, value: formatMoodSentiment(point.value) })}
                r="14"
              />
              {(index % labelStep === 0 || index === points.length - 1) && <text className="activity-axis-label" textAnchor="middle" x={x} y={height - 18}>{point.label}</text>}
            </g>
          );
        })}
      </svg>
      <ActivityTooltip tooltip={tooltip} />
    </div>
  );
}

function groupWeatherTrend(
  points: AnalyticsResponse["weather"]["trend"],
  metric: WeatherMetric,
  group: WeatherGroup,
) {
  const valueFor = (point: AnalyticsResponse["weather"]["trend"][number]) =>
    metric === "temperature"
      ? point.averageTempC
      : metric === "humidity"
        ? point.averageHumidity
        : point.averageWindKph;
  const periodFor = (date: string) => {
    if (group === "day") return date;
    if (group === "month") return date.slice(0, 7);
    const parsed = new Date(`${date}T12:00:00Z`);
    if (Number.isNaN(parsed.getTime())) return date;
    const dayOffset = (parsed.getUTCDay() + 6) % 7;
    parsed.setUTCDate(parsed.getUTCDate() - dayOffset);
    return `${parsed.toISOString().slice(0, 10)} week`;
  };
  const grouped = new Map<string, { weightedSum: number; weight: number; entryCount: number }>();
  for (const point of points) {
    const period = periodFor(point.date);
    const value = valueFor(point);
    const bucket = grouped.get(period) ?? { weightedSum: 0, weight: 0, entryCount: 0 };
    bucket.entryCount += point.entryCount;
    if (value !== null) {
      bucket.weightedSum += value * point.entryCount;
      bucket.weight += point.entryCount;
    }
    grouped.set(period, bucket);
  }
  return [...grouped.entries()].map(([date, bucket]) => ({
    date,
    value: bucket.weight > 0 ? bucket.weightedSum / bucket.weight : null,
    entryCount: bucket.entryCount,
  }));
}

function sentimentColor(value: number) {
  if (value > 0.35) return "#22c55e";
  if (value > 0) return "#86efac";
  if (value < -0.35) return "#ef4444";
  if (value < 0) return "#fca5a5";
  return "#94a3b8";
}

function correlationStrength(value: number) {
  const magnitude = Math.abs(value);
  const strength = magnitude >= 0.7 ? "strong" : magnitude >= 0.4 ? "moderate" : magnitude >= 0.2 ? "weak" : "very weak";
  return `${strength} ${value >= 0 ? "positive" : "negative"}`;
}

function formatTemperature(value: number | null) {
  return value === null ? "No data" : `${value.toFixed(1)} C`;
}

function formatTemperatureRange(min: number | null, max: number | null) {
  return min === null || max === null ? "No data" : `${min.toFixed(0)}-${max.toFixed(0)} C`;
}

export function ActivityTrends({ analytics }: { analytics: AnalyticsResponse }) {
  const [mode, setMode] = useState<ActivityTrendMode>("entries");
  const dailyEntryPoints = useMemo<BarPoint[]>(
    () =>
      analytics.dailyTrend.map((point) => ({
        key: point.date,
        label: point.date,
        value: point.entryCount,
        detail: `${point.entryCount} entries, ${point.wordCount} words`,
      })),
    [analytics.dailyTrend],
  );
  const dailyWordPoints = useMemo<LinePoint[]>(
    () =>
      analytics.dailyTrend.map((point) => ({
        key: point.date,
        label: point.date,
        value: point.wordCount,
        detail: `${point.wordCount} words, ${point.entryCount} entries`,
      })),
    [analytics.dailyTrend],
  );
  const wordsByHour = useMemo<BarPoint[]>(
    () =>
      analytics.hourlyTrend.map((point) => ({
        key: String(point.hour),
        label: point.label,
        value: point.wordCount,
        detail: `${point.wordCount} words, ${point.entryCount} entries`,
      })),
    [analytics.hourlyTrend],
  );
  const notesByHour = useMemo<BarPoint[]>(
    () =>
      analytics.hourlyTrend.map((point) => ({
        key: String(point.hour),
        label: point.label,
        value: point.entryCount,
        detail: `${point.entryCount} entries, ${point.wordCount} words`,
      })),
    [analytics.hourlyTrend],
  );
  const wordsByWeekday = useMemo<BarPoint[]>(
    () =>
      analytics.weekdayTrend.map((point) => ({
        key: String(point.dayNum),
        label: point.shortLabel,
        value: point.wordCount,
        detail: `${point.label}: ${point.wordCount} words, ${point.entryCount} entries`,
      })),
    [analytics.weekdayTrend],
  );
  const notesByWeekday = useMemo<BarPoint[]>(
    () =>
      analytics.weekdayTrend.map((point) => ({
        key: String(point.dayNum),
        label: point.shortLabel,
        value: point.entryCount,
        detail: `${point.label}: ${point.entryCount} entries, ${point.wordCount} words`,
      })),
    [analytics.weekdayTrend],
  );
  const locationPoints = useMemo<BarPoint[]>(
    () =>
      analytics.locationActivity.slice(0, 14).map((point) => ({
        key: point.label,
        label: truncateLabel(point.label, 18),
        value: point.count,
        detail: `${point.label}: ${point.count} entries`,
      })),
    [analytics.locationActivity],
  );
  const firstLastSeries = useMemo<LineSeries[]>(
    () => [
      {
        key: "first",
        label: "First Capsule",
        color: "#22c55e",
        points: analytics.writingWindow.days.map((point) => ({
          key: point.date,
          label: point.date,
          value: point.firstMinutes,
          detail: `${point.date}: first ${point.firstTime}`,
        })),
      },
      {
        key: "last",
        label: "Last Capsule",
        color: "#ec4899",
        points: analytics.writingWindow.days.map((point) => ({
          key: point.date,
          label: point.date,
          value: point.lastMinutes,
          detail: `${point.date}: last ${point.lastTime}`,
        })),
      },
    ],
    [analytics.writingWindow.days],
  );

  return (
    <div className="activity-trends">
      <div className="activity-tabs" role="tablist" aria-label="Activity trend charts">
        {ACTIVITY_TABS.map((tab) => (
          <button
            aria-selected={mode === tab.id}
            className={mode === tab.id ? "activity-tab activity-tab--active" : "activity-tab"}
            key={tab.id}
            onClick={() => setMode(tab.id)}
            role="tab"
            type="button"
          >
            {tab.label}
          </button>
        ))}
      </div>

      {mode === "entries" && (
        <SvgBarChart
          color="#3b82f6"
          emptyText="No entry data for this period."
          labelStep={dailyLabelStep(dailyEntryPoints.length)}
          points={dailyEntryPoints}
          valueLabel="entries"
        />
      )}
      {mode === "words" && (
        <SvgLineChart
          color="#0ea5e9"
          emptyText="No word-count data for this period."
          points={dailyWordPoints}
          valueFormatter={(value) => formatNumber(value)}
          valueLabel="words"
        />
      )}
      {mode === "writingWindow" && (
        <WritingWindowChart analytics={analytics} series={firstLastSeries} />
      )}
      {mode === "wordsHourly" && (
        <SvgBarChart
          color="#22c55e"
          emptyText="No hourly word data for this period."
          labelStep={2}
          points={wordsByHour}
          valueLabel="words"
        />
      )}
      {mode === "wordsWeekday" && (
        <SvgBarChart
          color="#16a34a"
          emptyText="No weekday word data for this period."
          points={wordsByWeekday}
          valueLabel="words"
        />
      )}
      {mode === "notesHourly" && (
        <SvgBarChart
          color="#f59e0b"
          emptyText="No note timing data for this period."
          labelStep={2}
          points={notesByHour}
          valueLabel="entries"
        />
      )}
      {mode === "notesWeekday" && (
        <SvgBarChart
          color="#14b8a6"
          emptyText="No weekday note data for this period."
          points={notesByWeekday}
          valueLabel="entries"
        />
      )}
      {mode === "location" && (
        <SvgBarChart
          color="#f43f5e"
          emptyText="No location data for this period."
          labelAngle={-32}
          points={locationPoints}
          valueLabel="entries"
        />
      )}
    </div>
  );
}

function WritingWindowChart({
  analytics,
  series,
}: {
  analytics: AnalyticsResponse;
  series: LineSeries[];
}) {
  const summary = analytics.writingWindow.summary;
  if (analytics.writingWindow.days.length === 0) {
    return <p className="muted activity-empty">No first/last capsule timing data for this period.</p>;
  }

  return (
    <div className="activity-window">
      <div className="activity-summary-grid">
        <ActivitySummary label="Avg first" value={summary.avgFirstTime ?? "No data"} />
        <ActivitySummary label="Avg last" value={summary.avgLastTime ?? "No data"} />
        <ActivitySummary label="Avg window" value={formatDuration(summary.avgSpanMinutes)} />
        <ActivitySummary
          label="Longest window"
          subvalue={summary.longestSpanDay?.date ?? "No active day"}
          value={
            summary.longestSpanDay
              ? formatDuration(summary.longestSpanDay.spanMinutes)
              : "No data"
          }
        />
      </div>
      <SvgLineChart
        emptyText="No first/last capsule timing data for this period."
        legend={series.map((item) => ({ color: item.color, label: item.label }))}
        series={series}
        timeScale
        valueFormatter={formatClock}
        valueLabel="time"
      />
    </div>
  );
}

function ActivitySummary({
  label,
  value,
  subvalue,
}: {
  label: string;
  value: ReactNode;
  subvalue?: ReactNode;
}) {
  return (
    <div className="activity-summary">
      <span>{label}</span>
      <strong>{value}</strong>
      {subvalue && <em>{subvalue}</em>}
    </div>
  );
}

function SvgBarChart({
  points,
  color,
  valueLabel,
  emptyText,
  labelStep = 1,
  labelAngle = 0,
}: {
  points: BarPoint[];
  color: string;
  valueLabel: string;
  emptyText: string;
  labelStep?: number;
  labelAngle?: number;
}) {
  const [tooltip, setTooltip] = useState<ActivityTooltipState | null>(null);

  if (points.length === 0 || !points.some((point) => point.value > 0)) {
    return <p className="muted activity-empty">{emptyText}</p>;
  }

  const maxValue = Math.max(1, ...points.map((point) => point.value));
  const ticks = numericTicks(maxValue);
  const innerWidth = CHART_WIDTH - BAR_MARGINS.left - BAR_MARGINS.right;
  const innerHeight = CHART_HEIGHT - BAR_MARGINS.top - BAR_MARGINS.bottom;
  const bandWidth = innerWidth / Math.max(1, points.length);
  const barWidth = Math.max(5, Math.min(52, bandWidth * 0.66));
  const yFor = (value: number) =>
    BAR_MARGINS.top + innerHeight - (value / Math.max(...ticks)) * innerHeight;
  const showTooltip = (
    event: ReactMouseEvent<Element>,
    point: BarPoint,
  ) => {
    setTooltip({
      ...tooltipPosition(event),
      color,
      detail: point.detail,
      title: point.label,
      value: `${formatNumber(point.value)} ${valueLabel}`,
    });
  };

  return (
    <div className="activity-chart-wrap" onMouseLeave={() => setTooltip(null)}>
      <svg
        aria-label={`${valueLabel} chart`}
        className="activity-chart"
        role="img"
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
      >
        <ChartGrid height={innerHeight} left={BAR_MARGINS.left} ticks={ticks} top={BAR_MARGINS.top} width={innerWidth} yFor={yFor} />
        {points.map((point, index) => {
          const x = BAR_MARGINS.left + index * bandWidth + (bandWidth - barWidth) / 2;
          const y = yFor(point.value);
          const height = BAR_MARGINS.top + innerHeight - y;
          const shouldShowLabel = index % labelStep === 0 || index === points.length - 1;
          return (
            <g key={point.key}>
              <rect
                className="activity-bar"
                fill={color}
                height={Math.max(0, height)}
                rx="4"
                width={barWidth}
                x={x}
                y={y}
              />
              <rect
                aria-label={`${point.label}: ${formatNumber(point.value)} ${valueLabel}`}
                className="activity-hit-target"
                height={innerHeight}
                onMouseEnter={(event) => showTooltip(event, point)}
                onMouseMove={(event) => showTooltip(event, point)}
                width={bandWidth}
                x={BAR_MARGINS.left + index * bandWidth}
                y={BAR_MARGINS.top}
              />
              {shouldShowLabel && (
                <text
                  className="activity-axis-label"
                  textAnchor={labelAngle === 0 ? "middle" : "end"}
                  transform={
                    labelAngle === 0
                      ? undefined
                      : `rotate(${labelAngle} ${x + barWidth / 2} ${CHART_HEIGHT - 18})`
                  }
                  x={x + barWidth / 2}
                  y={CHART_HEIGHT - 18}
                >
                  {point.label}
                </text>
              )}
            </g>
          );
        })}
        <ChartAxes height={innerHeight} left={BAR_MARGINS.left} top={BAR_MARGINS.top} width={innerWidth} />
        {ticks.map((tick) => (
          <text
            className="activity-axis-label"
            key={tick}
            textAnchor="end"
            x={BAR_MARGINS.left - 8}
            y={yFor(tick) + 4}
          >
            {formatNumber(tick)}
          </text>
        ))}
      </svg>
      <ActivityTooltip tooltip={tooltip} />
    </div>
  );
}

function SvgLineChart({
  points,
  series,
  color = "#0ea5e9",
  valueLabel,
  emptyText,
  timeScale = false,
  valueFormatter = formatNumber,
  legend,
  maxValueOverride,
}: {
  points?: LinePoint[];
  series?: LineSeries[];
  color?: string;
  valueLabel: string;
  emptyText: string;
  timeScale?: boolean;
  valueFormatter?: (value: number) => string;
  legend?: Array<{ color: string; label: string }>;
  maxValueOverride?: number;
}) {
  const [tooltip, setTooltip] = useState<ActivityTooltipState | null>(null);
  const resolvedSeries = series ?? [
    {
      key: "value",
      label: valueLabel,
      color,
      points: points ?? [],
    },
  ];
  const allPoints = resolvedSeries.flatMap((item) => item.points);
  const valuePoints = allPoints.filter((point): point is LinePoint & { value: number } => point.value !== null);

  if (valuePoints.length === 0) {
    return <p className="muted activity-empty">{emptyText}</p>;
  }

  const xPoints = resolvedSeries[0]?.points ?? [];
  const maxValue = timeScale ? 1439 : maxValueOverride ?? Math.max(1, ...valuePoints.map((point) => point.value));
  const ticks = timeScale ? [0, 360, 720, 1080, 1439] : numericTicks(maxValue);
  const domainMax = Math.max(...ticks);
  const innerWidth = CHART_WIDTH - LINE_MARGINS.left - LINE_MARGINS.right;
  const innerHeight = CHART_HEIGHT - LINE_MARGINS.top - LINE_MARGINS.bottom;
  const xFor = (index: number) =>
    xPoints.length <= 1
      ? LINE_MARGINS.left + innerWidth / 2
      : LINE_MARGINS.left + (index / (xPoints.length - 1)) * innerWidth;
  const yFor = (value: number) =>
    LINE_MARGINS.top + innerHeight - (value / domainMax) * innerHeight;
  const labelStep = dailyLabelStep(xPoints.length);
  const showTooltip = (
    event: ReactMouseEvent<Element>,
    point: LinePoint & { value: number },
    item: LineSeries,
  ) => {
    setTooltip({
      ...tooltipPosition(event),
      color: item.color,
      detail: point.detail,
      title: point.label,
      value: `${item.label}: ${valueFormatter(point.value)}`,
    });
  };

  return (
    <div className="activity-chart-wrap" onMouseLeave={() => setTooltip(null)}>
      <svg
        aria-label={`${valueLabel} chart`}
        className="activity-chart"
        role="img"
        viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
      >
        <ChartGrid height={innerHeight} left={LINE_MARGINS.left} ticks={ticks} top={LINE_MARGINS.top} width={innerWidth} yFor={yFor} />
        {timeScale && (
          <line
            className="activity-reference-line"
            x1={LINE_MARGINS.left}
            x2={LINE_MARGINS.left + innerWidth}
            y1={yFor(720)}
            y2={yFor(720)}
          />
        )}
        {resolvedSeries.map((item) => {
          const coordinates = item.points
            .map((point, index) =>
              point.value === null ? null : { ...point, x: xFor(index), y: yFor(point.value) },
            )
            .filter((point): point is LinePoint & { value: number; x: number; y: number } => point !== null);
          const path = coordinates.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ");
          return (
            <g key={item.key}>
              <path className="activity-line" d={path} stroke={item.color} />
              {coordinates.map((point) => (
                <g key={`${item.key}-${point.key}`}>
                  <circle className="activity-dot" cx={point.x} cy={point.y} fill={item.color} r="4" />
                  <circle
                    aria-label={`${point.label}: ${item.label} ${valueFormatter(point.value)}`}
                    className="activity-hit-target"
                    cx={point.x}
                    cy={point.y}
                    onMouseEnter={(event) => showTooltip(event, point, item)}
                    onMouseMove={(event) => showTooltip(event, point, item)}
                    r="12"
                  />
                </g>
              ))}
            </g>
          );
        })}
        <ChartAxes height={innerHeight} left={LINE_MARGINS.left} top={LINE_MARGINS.top} width={innerWidth} />
        {ticks.map((tick) => (
          <text
            className="activity-axis-label"
            key={tick}
            textAnchor="end"
            x={LINE_MARGINS.left - 8}
            y={yFor(tick) + 4}
          >
            {timeScale ? formatClock(tick) : formatNumber(tick)}
          </text>
        ))}
        {xPoints.map((point, index) =>
          index % labelStep === 0 || index === xPoints.length - 1 ? (
            <text
              className="activity-axis-label"
              key={point.key}
              textAnchor="middle"
              x={xFor(index)}
              y={CHART_HEIGHT - 18}
            >
              {point.label}
            </text>
          ) : null,
        )}
      </svg>
      {legend && (
        <div className="activity-legend">
          {legend.map((item) => (
            <span key={item.label} style={{ color: item.color }}>
              <i style={{ background: item.color }} />
              {item.label}
            </span>
          ))}
        </div>
      )}
      <ActivityTooltip tooltip={tooltip} />
    </div>
  );
}

function ActivityTooltip({ tooltip }: { tooltip: ActivityTooltipState | null }) {
  if (!tooltip) return null;

  return (
    <div
      className="activity-tooltip"
      role="tooltip"
      style={{ left: tooltip.x, top: tooltip.y }}
    >
      <span>
        <i style={{ background: tooltip.color }} />
        {tooltip.title}
      </span>
      <strong>{tooltip.value}</strong>
      {tooltip.detail && <em>{tooltip.detail}</em>}
    </div>
  );
}

function ChartGrid({
  left,
  top,
  width,
  height,
  ticks,
  yFor,
}: {
  left: number;
  top: number;
  width: number;
  height: number;
  ticks: number[];
  yFor: (value: number) => number;
}) {
  return (
    <g>
      {ticks.map((tick) => (
        <line
          className="activity-grid-line"
          key={tick}
          x1={left}
          x2={left + width}
          y1={yFor(tick)}
          y2={yFor(tick)}
        />
      ))}
      {Array.from({ length: 9 }).map((_, index) => {
        const x = left + (index / 8) * width;
        return <line className="activity-grid-line" key={index} x1={x} x2={x} y1={top} y2={top + height} />;
      })}
    </g>
  );
}

function tooltipPosition(event: ReactMouseEvent<Element>) {
  const tooltipWidth = 240;
  const tooltipHeight = 100;
  const offset = 14;
  return {
    x: Math.max(8, Math.min(event.clientX + offset, window.innerWidth - tooltipWidth - 8)),
    y: Math.max(8, Math.min(event.clientY + offset, window.innerHeight - tooltipHeight - 8)),
  };
}

function ChartAxes({
  left,
  top,
  width,
  height,
}: {
  left: number;
  top: number;
  width: number;
  height: number;
}) {
  return (
    <g>
      <line className="activity-axis-line" x1={left} x2={left} y1={top} y2={top + height} />
      <line className="activity-axis-line" x1={left} x2={left + width} y1={top + height} y2={top + height} />
    </g>
  );
}

function numericTicks(maxValue: number) {
  const roundedMax = niceMax(maxValue);
  const step = roundedMax / 4;
  return [0, step, step * 2, step * 3, roundedMax];
}

function niceMax(value: number) {
  if (value <= 4) return 4;
  const magnitude = 10 ** Math.floor(Math.log10(value));
  const normalized = value / magnitude;
  const nice =
    normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10;
  return nice * magnitude;
}

function dailyLabelStep(count: number) {
  return Math.max(1, Math.ceil(count / 12));
}

function formatClock(value: number) {
  const minutes = Math.max(0, Math.min(1439, Math.round(value)));
  const hour = Math.floor(minutes / 60);
  const minute = minutes % 60;
  return `${hour.toString().padStart(2, "0")}:${minute.toString().padStart(2, "0")}`;
}

function formatDuration(minutes: number | null | undefined) {
  if (minutes === null || minutes === undefined) return "No data";
  if (minutes <= 0) return "0 min";
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (hours > 0 && mins > 0) return `${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h`;
  return `${mins} min`;
}

function formatNumber(value: number) {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(value);
}

function truncateLabel(value: string, maxLength: number) {
  if (value.length <= maxLength) return value;
  return `${value.slice(0, maxLength - 1)}...`;
}
