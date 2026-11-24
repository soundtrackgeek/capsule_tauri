import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, test, vi } from "vitest";

import {
  DeleteEntryDialog,
  EntryAttachmentStrip,
  EntryCardContent,
  EntryDetail,
  EntryMini,
  EntryReadingContent,
  EntryStack,
} from "./entries";
import { makeEntry } from "../test/fixtures";
import type { ImageAttachment } from "../types";

function makeAttachment(overrides: Partial<ImageAttachment> = {}): ImageAttachment {
  return {
    attachmentId: 7,
    entryUuid: "entry_test42",
    mediaId: 12,
    position: 0,
    caption: "Window light",
    altText: "A test attachment thumbnail",
    createdAt: "2026-07-04 12:06",
    hash: "test-hash",
    mimeType: "image/jpeg",
    bytes: 12345,
    width: 800,
    height: 600,
    storageBackend: "local_fs",
    storageKey: "te/test-hash.jpg",
    deletedAt: null,
    thumbnailAvailable: true,
    originalAvailable: true,
    ...overrides,
  };
}

describe("entry components", () => {
  test("renders entry cards with number, metadata, summary, and image count", () => {
    render(<EntryCardContent entry={makeEntry()} />);

    expect(screen.getByRole("heading", { name: "Test entry" })).toBeInTheDocument();
    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.getByText("A compact summary for the test entry.")).toBeInTheDocument();
    expect(screen.getByText("Focused")).toBeInTheDocument();
    expect(screen.getByText("work")).toBeInTheDocument();
    expect(screen.getByTitle("Word count")).toHaveTextContent("12 words");
    expect(screen.getByTitle("Image attachments")).toHaveTextContent("2");
  });

  test("opens an entry thumbnail attachment", async () => {
    const user = userEvent.setup();
    const attachment = makeAttachment();
    const onOpen = vi.fn();

    render(<EntryAttachmentStrip attachments={[attachment]} onOpen={onOpen} />);

    expect(await screen.findByAltText("A test attachment thumbnail")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "View image 1" }));

    expect(onOpen).toHaveBeenCalledWith(attachment);
  });

  test("renders mini entries and empty/loading stack states", () => {
    const entry = makeEntry({ title: "Mini title" });
    const { rerender } = render(<EntryMini entry={entry} />);

    expect(screen.getByRole("heading", { name: "Mini title" })).toBeInTheDocument();
    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.getByTitle("Word count")).toHaveTextContent("12 words");

    rerender(<EntryStack entries={[]} loading={false} emptyText="Nothing here" />);
    expect(screen.getByText("Nothing here")).toBeInTheDocument();

    rerender(<EntryStack entries={[entry]} loading={false} />);
    expect(screen.getByRole("heading", { name: "Mini title" })).toBeInTheDocument();
  });

  test("wires entry detail actions to callbacks", async () => {
    const user = userEvent.setup();
    const entry = makeEntry();
    const onEdit = vi.fn();
    const onContinue = vi.fn();
    const onDelete = vi.fn();
    const onEntryAction = vi.fn();
    const onExport = vi.fn();
    const onLoadHistory = vi.fn();

    render(
      <EntryDetail
        entry={entry}
        entryHistory={null}
        historyLoading={false}
        loading={false}
        mutating={false}
        onContinue={onContinue}
        onDelete={onDelete}
        onEdit={onEdit}
        onEntryAction={onEntryAction}
        onExport={onExport}
        onLoadHistory={onLoadHistory}
      />,
    );

    expect(screen.getByRole("heading", { name: "Test entry" })).toBeInTheDocument();
    expect(screen.getByText("Tromso, Norway")).toBeInTheDocument();
    expect(screen.getByText("Words").nextElementSibling).toHaveTextContent("12 words");
    expect(screen.queryByText("manual")).not.toBeInTheDocument();

    await user.click(screen.getByTitle("Star"));
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Continue" }));
    await user.click(screen.getByRole("button", { name: "MD" }));
    await user.click(screen.getByRole("button", { name: "Load" }));
    await user.click(screen.getByTitle("Delete entry"));

    expect(onEntryAction).toHaveBeenCalledWith(entry, "star");
    expect(onEdit).toHaveBeenCalledWith(entry);
    expect(onContinue).toHaveBeenCalledWith(entry);
    expect(onExport).toHaveBeenCalledWith(entry, "markdown");
    expect(onLoadHistory).toHaveBeenCalledWith(entry);
    expect(onDelete).toHaveBeenCalledWith(entry);
  });

  test("renders entry detail with authored line breaks", () => {
    const text = "First paragraph.\n\nSecond paragraph.\nThird line.";
    const entry = makeEntry({
      attachmentCount: 0,
      contentFormat: "plain",
      text,
      textPlain: "First paragraph. Second paragraph. Third line.",
    });

    const { container } = render(
      <EntryDetail
        entry={entry}
        entryHistory={null}
        historyLoading={false}
        loading={false}
        mutating={false}
        onContinue={vi.fn()}
        onDelete={vi.fn()}
        onEdit={vi.fn()}
        onEntryAction={vi.fn()}
        onExport={vi.fn()}
        onLoadHistory={vi.fn()}
      />,
    );

    expect(container.querySelector(".entry-body")?.textContent).toBe(text);
    expect(screen.queryByRole("button", { name: "Raw" })).not.toBeInTheDocument();
  });

  test("formats Markdown while preserving the original text and blocking unsafe content", async () => {
    const user = userEvent.setup();
    const text = [
      "## A day outside",
      "",
      "First **line**\nSecond line with a [map](https://example.com/map).",
      "",
      "- A list item",
      "",
      "[Blocked link](javascript:alert%281%29)",
      "![Remote photo](https://example.com/photo.jpg)",
      "",
      "<iframe src='https://example.com/tracker'></iframe>",
    ].join("\n");
    const { container } = render(
      <EntryReadingContent entry={makeEntry({ attachmentCount: 0, text })} />,
    );

    expect(await screen.findByRole("heading", { name: "A day outside" })).toBeInTheDocument();
    expect(screen.getByText("line", { selector: "strong" })).toBeInTheDocument();
    expect(container.querySelector(".markdown-content br")).toBeInTheDocument();
    expect(screen.getByRole("listitem")).toHaveTextContent("A list item");
    expect(screen.getByRole("link", { name: "map" })).toHaveAttribute("href", "https://example.com/map");
    expect(screen.queryByRole("link", { name: "Blocked link" })).not.toBeInTheDocument();
    expect(screen.getByText(/Remote photo/)).toBeInTheDocument();
    expect(container.querySelector("iframe, img")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Raw" }));
    expect(container.querySelector(".entry-body")?.textContent).toBe(text);
    await user.click(screen.getByRole("button", { name: "Formatted" }));
    expect(screen.getByRole("heading", { name: "A day outside" })).toBeInTheDocument();
  });

  test("renders entry detail as an embedded reader", async () => {
    const entry = makeEntry();
    const { container } = render(
      <EntryDetail
        embedded
        entry={entry}
        entryHistory={null}
        historyLoading={false}
        loading={false}
        mutating={false}
        onContinue={vi.fn()}
        onDelete={vi.fn()}
        onEdit={vi.fn()}
        onEntryAction={vi.fn()}
        onExport={vi.fn()}
        onLoadHistory={vi.fn()}
      />,
    );

    expect(container.querySelector(".entry-reader")).toBeInTheDocument();
    expect(container.querySelector(".detail-panel")).not.toBeInTheDocument();
    expect(
      await screen.findByText("A focused test entry with enough body text to render useful previews."),
    ).toBeInTheDocument();
  });

  test("confirms or cancels destructive delete dialog", async () => {
    const user = userEvent.setup();
    const onCancel = vi.fn();
    const onConfirm = vi.fn();

    render(
      <DeleteEntryDialog
        deleting={false}
        entry={makeEntry()}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />,
    );

    expect(screen.getByRole("dialog", { name: "Test entry" })).toBeInTheDocument();

    await user.click(screen.getAllByRole("button", { name: "Cancel" }).at(-1)!);
    await user.click(screen.getByRole("button", { name: "Yes, I want to delete" }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).toHaveBeenCalledTimes(1);
  });
});
