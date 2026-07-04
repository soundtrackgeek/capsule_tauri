import type { ReactNode } from "react";
import type { DatabaseStatus } from "../types";

type PanelProps = {
  icon: ReactNode;
  title: string;
  action?: ReactNode;
  children: ReactNode;
};

export function Panel({ icon, title, action, children }: PanelProps) {
  return (
    <article className="panel">
      <div className="panel-header">
        <div className="panel-title">
          {icon}
          <h3>{title}</h3>
        </div>
        {action}
      </div>
      {children}
    </article>
  );
}

type DetailProps = {
  label: string;
  value: ReactNode;
};

export function Detail({ label, value }: DetailProps) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{value}</dd>
    </>
  );
}

export function Metric({ label, value }: DetailProps) {
  return (
    <div className="metric">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

export function SkeletonList({ compact = false }: { compact?: boolean }) {
  return (
    <div className={compact ? "skeleton-list skeleton-list--compact" : "skeleton-list"}>
      {Array.from({ length: compact ? 3 : 5 }).map((_, index) => (
        <div className="skeleton-card" key={index}>
          <div className="skeleton skeleton-title" />
          <div className="skeleton skeleton-line" />
          <div className="skeleton skeleton-line skeleton-line--short" />
        </div>
      ))}
    </div>
  );
}

export function WarningList({ warnings }: { warnings: string[] }) {
  if (warnings.length === 0) {
    return null;
  }

  return (
    <ul className="warning-list phase6-warning-list">
      {warnings.map((warning) => (
        <li key={warning}>{warning}</li>
      ))}
    </ul>
  );
}

export function UnavailableState({
  icon,
  label,
  status,
}: {
  icon: ReactNode;
  label: string;
  status: DatabaseStatus | null;
}) {
  return (
    <section className="state-panel">
      {icon}
      <h3>{label}</h3>
      <p>{status?.dbPath ?? "No database path resolved."}</p>
    </section>
  );
}
