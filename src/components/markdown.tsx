import { lazy, Suspense } from "react";

const MarkdownRenderer = lazy(() => import("./markdown-renderer"));

export function MarkdownContent({ text }: { text: string }) {
  return (
    <Suspense fallback={<div role="status">Loading formatted text…</div>}>
      <MarkdownRenderer text={text} />
    </Suspense>
  );
}
