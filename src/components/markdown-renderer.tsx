import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import Markdown, { type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";

const remarkPlugins = [remarkGfm, remarkBreaks];
const components: Components = {
  a: ({ href, children }) => {
    if (!href) {
      return <>{children}</>;
    }

    return (
      <a
        href={href}
        onClick={(event) => {
          if (isTauri()) {
            event.preventDefault();
            void openUrl(href).catch((error) => {
              console.error("Unable to open journal link", error);
            });
          }
        }}
        rel="noreferrer"
        target="_blank"
      >
        {children}
      </a>
    );
  },
  // The journal displays local attachments without making requests to image hosts.
  img: ({ alt }) => alt ? <>{alt}</> : null,
  table: ({ children }) => (
    <div className="markdown-table-scroll">
      <table>{children}</table>
    </div>
  ),
};

export default function MarkdownRenderer({ text }: { text: string }) {
  return (
    <div className="markdown-content">
      <Markdown
        components={components}
        remarkPlugins={remarkPlugins}
        skipHtml
        urlTransform={(url) => /^(?:https?:\/\/|mailto:)/i.test(url) ? url : ""}
      >
        {text}
      </Markdown>
    </div>
  );
}
