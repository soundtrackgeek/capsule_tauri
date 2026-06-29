import type { ReactNode } from "react";

type StatusPillProps = {
  tone: "good" | "warn" | "neutral";
  children: ReactNode;
};

export function StatusPill({ tone, children }: StatusPillProps) {
  return <span className={`status-pill status-pill--${tone}`}>{children}</span>;
}
