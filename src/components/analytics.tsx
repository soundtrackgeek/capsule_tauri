import { useMemo, useState, type ReactNode } from "react";
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

  return (
    <div className="activity-chart-wrap">
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
              >
                <title>{point.detail ?? `${point.label}: ${formatNumber(point.value)} ${valueLabel}`}</title>
              </rect>
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
}: {
  points?: LinePoint[];
  series?: LineSeries[];
  color?: string;
  valueLabel: string;
  emptyText: string;
  timeScale?: boolean;
  valueFormatter?: (value: number) => string;
  legend?: Array<{ color: string; label: string }>;
}) {
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
  const maxValue = timeScale ? 1439 : Math.max(1, ...valuePoints.map((point) => point.value));
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

  return (
    <div className="activity-chart-wrap">
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
                <circle className="activity-dot" cx={point.x} cy={point.y} fill={item.color} key={`${item.key}-${point.key}`} r="4">
                  <title>
                    {point.detail ?? `${point.label}: ${valueFormatter(point.value)} ${valueLabel}`}
                  </title>
                </circle>
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
