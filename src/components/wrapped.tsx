import {
  BarChart3,
  BookOpen,
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Clock3,
  Heart,
  MapPin,
  NotebookPen,
  PenLine,
  RefreshCw,
  Sparkles,
  Tags,
  Trophy,
  type LucideIcon,
} from "lucide-react";
import type {
  DatabaseStatus,
  WrappedActivityPoint,
  WrappedBadge,
  WrappedChartCountPoint,
  WrappedMetricComparison,
  WrappedPeriod,
  WrappedResponse,
} from "../types";

const PERIODS: Array<{ id: WrappedPeriod; label: string }> = [
  { id: "week", label: "Week" },
  { id: "month", label: "Month" },
  { id: "year", label: "Year" },
];

const TAG_COLORS = [
  "#f97316",
  "#f59e0b",
  "#22c55e",
  "#06b6d4",
  "#3b82f6",
  "#8b5cf6",
  "#ec4899",
  "#14b8a6",
];

const MOOD_COLORS = [
  "#38bdf8",
  "#22c55e",
  "#f59e0b",
  "#a78bfa",
  "#f97316",
  "#f43f5e",
  "#14b8a6",
  "#64748b",
  "#84cc16",
  "#e879f9",
];

type WrappedViewProps = {
  status: DatabaseStatus | null;
  report: WrappedResponse | null;
  period: WrappedPeriod;
  loading: boolean;
  onAnchorChange: (anchor: string) => void;
  onPeriodChange: (period: WrappedPeriod) => void;
  onRefresh: () => void;
  onWrite: () => void;
};

export function WrappedView({
  status,
  report,
  period,
  loading,
  onAnchorChange,
  onPeriodChange,
  onRefresh,
  onWrite,
}: WrappedViewProps) {
  if (status && (!status.dbExists || !status.readable)) {
    return (
      <section className="state-panel">
        <Sparkles size={22} />
        <h3>Wrapped is not available</h3>
        <p>{status.security.message ?? "Open Settings to confirm the active database path."}</p>
        <code>{status.dbPath}</code>
      </section>
    );
  }

  return (
    <section className="wrapped-workspace" aria-label="Capsule Wrapped">
      <header className="wrapped-intro">
        <div>
          <div className="wrapped-intro-title">
            <Sparkles aria-hidden="true" size={22} />
            <h3>Capsule Wrapped</h3>
          </div>
          <p>Completed-period retrospectives for your journal habit.</p>
        </div>
        <div className="wrapped-period-tabs" aria-label="Wrapped period" role="tablist">
          {PERIODS.map((item) => (
            <button
              aria-selected={period === item.id}
              className={
                period === item.id
                  ? "wrapped-period-tab wrapped-period-tab--active"
                  : "wrapped-period-tab"
              }
              key={item.id}
              onClick={() => onPeriodChange(item.id)}
              role="tab"
              type="button"
            >
              {item.label}
            </button>
          ))}
        </div>
      </header>

      {loading && <WrappedSkeleton />}
      {!loading && !report && (
        <div className="wrapped-empty">
          <Sparkles aria-hidden="true" size={28} />
          <h3>Your retrospective is ready to load</h3>
          <p>Refresh Wrapped to gather the latest completed-period stats.</p>
          <button className="secondary-button" onClick={onRefresh} type="button">
            <RefreshCw size={17} />
            Refresh Wrapped
          </button>
        </div>
      )}
      {report && (
        <WrappedReport
          onAnchorChange={onAnchorChange}
          onWrite={onWrite}
          period={period}
          report={report}
        />
      )}
    </section>
  );
}

function WrappedReport({
  report,
  period,
  onAnchorChange,
  onWrite,
}: {
  report: WrappedResponse;
  period: WrappedPeriod;
  onAnchorChange: (anchor: string) => void;
  onWrite: () => void;
}) {
  const hasEntries = report.summary.entries > 0;
  const highlights = [
    {
      icon: Tags,
      label: "Top tag",
      value: report.highlights.topTag?.label,
    },
    {
      icon: Heart,
      label: "Top mood",
      value: report.highlights.topMood?.label,
    },
    {
      icon: CalendarDays,
      label: "Peak day",
      value: report.highlights.topWeekday?.label,
    },
    {
      icon: Clock3,
      label: "Peak hour",
      value: report.highlights.topHour?.label,
    },
    {
      icon: MapPin,
      label: "Top place",
      value: report.highlights.topLocation?.label,
    },
  ].filter((item): item is { icon: LucideIcon; label: string; value: string } =>
    Boolean(item.value),
  );

  return (
    <div className="wrapped-report">
      <section className="wrapped-hero">
        <div className="wrapped-hero-main">
          <div>
            <p className="wrapped-period-pill">
              <Sparkles aria-hidden="true" size={15} />
              {report.range.label}
            </p>
            <h3>{report.title}</h3>
            <p className="wrapped-hero-summary">
              {hasEntries
                ? `${formatInt(report.summary.entries)} ${report.summary.entries === 1 ? "entry" : "entries"} across ${formatInt(report.summary.activeDays)} active ${report.summary.activeDays === 1 ? "day" : "days"}, averaging ${formatDecimal(report.summary.avgEntriesPerActiveDay)} ${report.summary.avgEntriesPerActiveDay === 1 ? "entry" : "entries"} each day you showed up.`
                : `No visible entries landed in this completed ${period}. You can still browse older periods or start writing toward the next wrap.`}
            </p>
          </div>

          <div className="wrapped-nav">
            <button
              className="secondary-button secondary-button--small"
              onClick={() => onAnchorChange(report.navigation.previousAnchor)}
              title={report.navigation.previousLabel}
              type="button"
            >
              <ChevronLeft size={16} />
              Older
            </button>
            <button
              className="secondary-button secondary-button--small"
              disabled={!report.navigation.nextAnchor}
              onClick={() =>
                report.navigation.nextAnchor && onAnchorChange(report.navigation.nextAnchor)
              }
              title={report.navigation.nextLabel ?? "Already showing the latest completed period"}
              type="button"
            >
              Newer
              <ChevronRight size={16} />
            </button>
          </div>

          {highlights.length > 0 && (
            <div className="wrapped-highlight-grid">
              {highlights.map(({ icon: Icon, label, value }) => (
                <article className="wrapped-highlight" key={label}>
                  <p>
                    <Icon aria-hidden="true" size={15} />
                    {label}
                  </p>
                  <strong>{value}</strong>
                </article>
              ))}
            </div>
          )}
        </div>

        <aside className="wrapped-callouts">
          <h4>
            <Sparkles aria-hidden="true" size={17} />
            Lifetime callouts in this period
          </h4>
          {report.personalBestBadges.length > 0 ? (
            <div className="wrapped-badge-list">
              {report.personalBestBadges.map((badge) => (
                <WrappedBadgeCard badge={badge} key={badge.id} />
              ))}
            </div>
          ) : (
            <p>No lifetime personal-best moments landed inside this wrapped period.</p>
          )}
        </aside>
      </section>

      {!hasEntries ? (
        <section className="wrapped-empty">
          <Sparkles aria-hidden="true" size={28} />
          <h3>Nothing to wrap here yet</h3>
          <p>
            This completed {period} has no visible entries. Browse farther back or start writing
            toward your next retrospective.
          </p>
          <div className="wrapped-empty-actions">
            <button
              className="secondary-button"
              onClick={() => onAnchorChange(report.navigation.previousAnchor)}
              type="button"
            >
              <ChevronLeft size={17} />
              Browse older {period}
            </button>
            <button className="primary-button" onClick={onWrite} type="button">
              <NotebookPen size={17} />
              Write a new entry
            </button>
          </div>
        </section>
      ) : (
        <>
          <section className="wrapped-metrics" aria-label="Period totals">
            <WrappedMetric
              accent="amber"
              comparison={report.comparison.entries}
              label="Entries"
              value={formatInt(report.summary.entries)}
            />
            <WrappedMetric
              accent="sky"
              comparison={report.comparison.words}
              label="Words"
              value={formatInt(report.summary.words)}
            />
            <WrappedMetric
              accent="rose"
              comparison={report.comparison.activeDays}
              label="Active days"
              value={formatInt(report.summary.activeDays)}
            />
            <WrappedMetric
              accent="emerald"
              description={`${formatDecimal(report.summary.avgEntriesPerActiveDay)} ${
                report.summary.avgEntriesPerActiveDay === 1 ? "entry" : "entries"
              } per active day`}
              label="Avg words / entry"
              value={formatDecimal(report.summary.avgWordsPerEntry)}
            />
            <WrappedMetric
              accent="cyan"
              comparison={report.comparison.healthScore}
              label="Health score"
              value={formatInt(report.summary.healthScore)}
            />
            <WrappedMetric
              accent="violet"
              description={`Best run in ${report.range.label}`}
              label="Longest streak"
              value={`${formatInt(report.summary.longestStreak)} ${report.summary.longestStreak === 1 ? "day" : "days"}`}
            />
          </section>

          {report.insights.length > 0 && (
            <WrappedCardSection
              description={`Deterministic takeaways from this completed ${period}.`}
              items={report.insights}
              title="Insights"
            />
          )}

          {report.funFacts.length > 0 && (
            <WrappedCardSection
              description="Little standouts from this wrapped period."
              items={report.funFacts}
              title="Fun Facts"
            />
          )}

          <section className="wrapped-chart-section" aria-label="Wrapped charts">
            <div className="wrapped-section-heading">
              <div>
                <h3>Charts</h3>
                <p>Activity, tags, and moods from {report.range.label}.</p>
              </div>
            </div>

            <article className="wrapped-chart-card wrapped-activity-card">
              <h4>
                <BarChart3 aria-hidden="true" size={19} />
                Activity
              </h4>
              <WrappedActivityChart
                granularity={report.charts.activityGranularity}
                points={report.charts.activity}
              />
            </article>

            <div className="wrapped-chart-grid">
              <article className="wrapped-chart-card">
                <h4>Top Tags</h4>
                <WrappedTagChart points={report.charts.topTags} />
              </article>
              <article className="wrapped-chart-card">
                <h4>Mood Distribution</h4>
                <WrappedMoodChart points={report.charts.moodDistribution} />
              </article>
            </div>
          </section>
        </>
      )}
    </div>
  );
}

function WrappedBadgeCard({ badge }: { badge: WrappedBadge }) {
  const iconMap: Record<WrappedBadge["id"], LucideIcon> = {
    best_notes_day: Trophy,
    best_words_day: PenLine,
    most_tags_in_post: Tags,
    longest_entry: BookOpen,
  };
  const Icon = iconMap[badge.id];
  return (
    <article className="wrapped-badge">
      <span aria-hidden="true">
        <Icon size={18} />
      </span>
      <div>
        <p>{badge.title}</p>
        <strong>{badge.value}</strong>
        <small>{badge.detail}</small>
      </div>
    </article>
  );
}

function WrappedMetric({
  label,
  value,
  comparison,
  description,
  accent,
}: {
  label: string;
  value: string;
  comparison?: WrappedMetricComparison;
  description?: string;
  accent: "amber" | "sky" | "rose" | "emerald" | "cyan" | "violet";
}) {
  return (
    <article className={`wrapped-metric wrapped-metric--${accent}`}>
      <p>{label}</p>
      <strong>{value}</strong>
      <span>{comparison ? formatComparison(comparison) : description}</span>
    </article>
  );
}

function WrappedCardSection({
  title,
  description,
  items,
}: {
  title: string;
  description: string;
  items: Array<{ kind: string; title: string; body: string }>;
}) {
  return (
    <section className="wrapped-card-section">
      <div className="wrapped-section-heading">
        <div>
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </div>
      <div className="wrapped-fact-grid">
        {items.map((item) => (
          <article className="wrapped-fact-card" key={item.kind}>
            <p>{item.title}</p>
            <strong>{item.body}</strong>
          </article>
        ))}
      </div>
    </section>
  );
}

function WrappedActivityChart({
  points,
  granularity,
}: {
  points: WrappedActivityPoint[];
  granularity: "day" | "month";
}) {
  if (!points.some((point) => point.entries > 0 || point.words > 0)) {
    return <p className="muted wrapped-chart-empty">No activity in this period.</p>;
  }

  const width = 960;
  const height = 320;
  const margin = { top: 22, right: 56, bottom: 54, left: 48 };
  const innerWidth = width - margin.left - margin.right;
  const innerHeight = height - margin.top - margin.bottom;
  const maxEntries = Math.max(1, ...points.map((point) => point.entries));
  const maxWords = Math.max(1, ...points.map((point) => point.words));
  const entryTickStep = Math.max(1, Math.ceil(maxEntries / 4));
  const entryTicks = Array.from(
    new Set([
      ...Array.from(
        { length: Math.floor(maxEntries / entryTickStep) + 1 },
        (_, index) => index * entryTickStep,
      ),
      maxEntries,
    ]),
  ).sort((a, b) => a - b);
  const band = innerWidth / Math.max(1, points.length);
  const barWidth = Math.max(5, Math.min(42, band * 0.62));
  const xFor = (index: number) => margin.left + band * index + band / 2;
  const entryY = (value: number) =>
    margin.top + innerHeight - (value / maxEntries) * innerHeight;
  const wordY = (value: number) =>
    margin.top + innerHeight - (value / maxWords) * innerHeight;
  const labelStep = Math.max(1, Math.ceil(points.length / 10));
  const path = points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${xFor(index)} ${wordY(point.words)}`)
    .join(" ");

  return (
    <div className="wrapped-activity-chart">
      <svg
        aria-label="Entries and words over the wrapped period"
        role="img"
        viewBox={`0 0 ${width} ${height}`}
      >
        {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
          const y = margin.top + innerHeight - ratio * innerHeight;
          return (
            <g key={ratio}>
              <line
                className="wrapped-chart-grid-line"
                x1={margin.left}
                x2={margin.left + innerWidth}
                y1={y}
                y2={y}
              />
              <text
                className="wrapped-chart-axis-label wrapped-chart-axis-label--words"
                textAnchor="start"
                x={width - margin.right + 9}
                y={y + 4}
              >
                {Math.round(maxWords * ratio)}
              </text>
            </g>
          );
        })}
        {entryTicks.map((value) => {
          const y = entryY(value);
          return (
            <text
              className="wrapped-chart-axis-label"
              key={value}
              textAnchor="end"
              x={margin.left - 9}
              y={y + 4}
            >
              {value}
            </text>
          );
        })}
        {points.map((point, index) => {
          const x = xFor(index);
          const y = entryY(point.entries);
          const barHeight = margin.top + innerHeight - y;
          const showLabel = index % labelStep === 0 || index === points.length - 1;
          return (
            <g key={point.period}>
              <rect
                className="wrapped-activity-bar"
                height={Math.max(0, barHeight)}
                rx="4"
                width={barWidth}
                x={x - barWidth / 2}
                y={y}
              >
                <title>{`${formatActivityDate(point.period, granularity)}: ${point.entries} ${
                  point.entries === 1 ? "entry" : "entries"
                }, ${point.words} ${point.words === 1 ? "word" : "words"}`}</title>
              </rect>
              {showLabel && (
                <text
                  className="wrapped-chart-axis-label"
                  textAnchor="middle"
                  x={x}
                  y={height - 20}
                >
                  {formatActivityAxis(point.period, granularity)}
                </text>
              )}
            </g>
          );
        })}
        <path className="wrapped-activity-line" d={path} />
        {points.map((point, index) => (
          <circle
            className="wrapped-activity-dot"
            cx={xFor(index)}
            cy={wordY(point.words)}
            key={point.period}
            r="4"
          >
            <title>{`${formatActivityDate(point.period, granularity)}: ${point.words} words`}</title>
          </circle>
        ))}
      </svg>
      <div className="wrapped-chart-legend" aria-hidden="true">
        <span className="wrapped-chart-legend--entries">
          <i />
          Entries
        </span>
        <span className="wrapped-chart-legend--words">
          <i />
          Words
        </span>
      </div>
    </div>
  );
}

function WrappedTagChart({ points }: { points: WrappedChartCountPoint[] }) {
  if (points.length === 0) {
    return <p className="muted wrapped-chart-empty">No tag data in this wrapped period.</p>;
  }
  const max = Math.max(...points.map((point) => point.count), 1);
  return (
    <div className="wrapped-tag-chart">
      {points.map((point, index) => (
        <div className="wrapped-tag-row" key={point.label}>
          <span title={point.label}>{point.label}</span>
          <div>
            <i
              style={{
                background: TAG_COLORS[index % TAG_COLORS.length],
                width: `${(point.count / max) * 100}%`,
              }}
            />
          </div>
          <strong>{formatInt(point.count)}</strong>
        </div>
      ))}
    </div>
  );
}

function WrappedMoodChart({ points }: { points: WrappedChartCountPoint[] }) {
  if (points.length === 0) {
    return <p className="muted wrapped-chart-empty">No mood data in this wrapped period.</p>;
  }
  const total = points.reduce((sum, point) => sum + point.count, 0);
  const radius = 74;
  const circumference = 2 * Math.PI * radius;
  let offset = 0;
  return (
    <div className="wrapped-mood-chart">
      <svg aria-label="Mood distribution" role="img" viewBox="0 0 210 210">
        <circle className="wrapped-donut-track" cx="105" cy="105" fill="none" r={radius} />
        {points.map((point, index) => {
          const length = (point.count / total) * circumference;
          const dashOffset = -offset;
          offset += length;
          return (
            <circle
              cx="105"
              cy="105"
              fill="none"
              key={point.label}
              r={radius}
              stroke={MOOD_COLORS[index % MOOD_COLORS.length]}
              strokeDasharray={`${length} ${circumference - length}`}
              strokeDashoffset={dashOffset}
              strokeWidth="38"
              transform="rotate(-90 105 105)"
            >
              <title>{`${point.label}: ${point.count} (${Math.round((point.count / total) * 100)}%)`}</title>
            </circle>
          );
        })}
        <text className="wrapped-donut-total" textAnchor="middle" x="105" y="101">
          {formatInt(total)}
        </text>
        <text className="wrapped-donut-caption" textAnchor="middle" x="105" y="122">
          check-ins
        </text>
      </svg>
      <div className="wrapped-mood-legend">
        {points.map((point, index) => (
          <span key={point.label}>
            <i style={{ background: MOOD_COLORS[index % MOOD_COLORS.length] }} />
            <b>{point.label}</b>
            <em>{formatInt(point.count)}</em>
          </span>
        ))}
      </div>
    </div>
  );
}

function WrappedSkeleton() {
  return (
    <div aria-label="Loading Wrapped" className="wrapped-skeleton" role="status">
      <div className="wrapped-skeleton-hero" />
      <div className="wrapped-skeleton-grid">
        {Array.from({ length: 6 }).map((_, index) => (
          <div key={index} />
        ))}
      </div>
    </div>
  );
}

function formatComparison(comparison: WrappedMetricComparison) {
  if (comparison.delta === 0) return "Matched the previous period";
  const prefix = comparison.delta > 0 ? "+" : "";
  const percent =
    comparison.pctChange === null ? "" : ` (${prefix}${comparison.pctChange.toLocaleString()}%)`;
  return `${prefix}${comparison.delta.toLocaleString()} vs previous${percent}`;
}

function formatInt(value: number) {
  return new Intl.NumberFormat().format(value);
}

function formatDecimal(value: number) {
  return new Intl.NumberFormat(undefined, {
    maximumFractionDigits: 1,
    minimumFractionDigits: value % 1 === 0 ? 0 : 1,
  }).format(value);
}

function formatActivityAxis(value: string, granularity: "day" | "month") {
  const date = new Date(`${granularity === "month" ? `${value}-01` : value}T12:00:00`);
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: granularity === "day" ? "numeric" : undefined,
  });
}

function formatActivityDate(value: string, granularity: "day" | "month") {
  const date = new Date(`${granularity === "month" ? `${value}-01` : value}T12:00:00`);
  return date.toLocaleDateString(undefined, {
    month: "long",
    day: granularity === "day" ? "numeric" : undefined,
    year: "numeric",
  });
}
