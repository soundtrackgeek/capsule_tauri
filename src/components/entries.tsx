import {
  Download,
  Edit3,
  Eye,
  EyeOff,
  FileText,
  History,
  Image as ImageIcon,
  MapPin,
  Pin,
  PinOff,
  Search,
  Star,
  Tags,
  Trash2,
  TriangleAlert,
  X,
} from "lucide-react";
import { formatDateTime, formatEntryNumber } from "../lib/format";
import type { Entry, EntryHistoryResponse, ExportFormat } from "../types";
import { Detail, SkeletonList } from "./ui";

type EntryDetailProps = {
  entry: Entry | null;
  entryHistory: EntryHistoryResponse | null;
  embedded?: boolean;
  historyLoading: boolean;
  loading: boolean;
  mutating: boolean;
  onEdit: (entry: Entry) => void;
  onContinue: (entry: Entry) => void;
  onDelete: (entry: Entry) => void;
  onEntryAction: (entry: Entry, action: "star" | "pin" | "hide" | "unhide") => void;
  onExport: (entry: Entry, format: ExportFormat) => void;
  onLoadHistory: (entry: Entry) => void;
};

function formatWeatherTemperature(location: NonNullable<Entry["location"]>) {
  if (location.weatherTempC !== null) {
    return `${location.weatherTempC.toFixed(1)} C`;
  }

  if (location.weatherTempF !== null) {
    return `${location.weatherTempF.toFixed(1)} F`;
  }

  return null;
}

export function EntryDetail({
  entry,
  entryHistory,
  embedded = false,
  historyLoading,
  loading,
  mutating,
  onEdit,
  onContinue,
  onDelete,
  onEntryAction,
  onExport,
  onLoadHistory,
}: EntryDetailProps) {
  const Wrapper = embedded ? "div" : "aside";
  const panelClassName = embedded ? "entry-reader" : "detail-panel";
  const emptyClassName = embedded ? "entry-reader entry-reader--empty" : "detail-panel detail-panel--empty";

  if (loading) {
    return (
      <Wrapper className={panelClassName}>
        <div className="skeleton skeleton-title" />
        <div className="skeleton skeleton-line" />
        <div className="skeleton skeleton-block" />
      </Wrapper>
    );
  }

  if (!entry) {
    return (
      <Wrapper className={emptyClassName}>
        <Search size={22} />
        <h3>No entry selected</h3>
        <p>Select an entry to inspect it.</p>
      </Wrapper>
    );
  }

  const weatherTemperature = entry.location ? formatWeatherTemperature(entry.location) : null;

  return (
    <Wrapper className={panelClassName}>
      <div className="entry-detail-heading">
        <p className="eyebrow">{formatDateTime(entry.createdAt)}</p>
        <h3>{entry.title || entry.textPlain.slice(0, 72) || "Untitled entry"}</h3>
      </div>

      <div className="entry-action-bar">
        <button
          className={entry.starred ? "icon-button icon-button--active" : "icon-button"}
          disabled={mutating}
          onClick={() => onEntryAction(entry, "star")}
          title={entry.starred ? "Unstar" : "Star"}
          type="button"
        >
          <Star size={17} />
        </button>
        <button
          className={entry.pinned ? "icon-button icon-button--active" : "icon-button"}
          disabled={mutating}
          onClick={() => onEntryAction(entry, "pin")}
          title={entry.pinned ? "Unpin" : "Pin"}
          type="button"
        >
          {entry.pinned ? <PinOff size={17} /> : <Pin size={17} />}
        </button>
        <button
          className="icon-button"
          disabled={mutating}
          onClick={() => onEntryAction(entry, entry.hidden ? "unhide" : "hide")}
          title={entry.hidden ? "Unhide" : "Hide"}
          type="button"
        >
          {entry.hidden ? <Eye size={17} /> : <EyeOff size={17} />}
        </button>
        <button className="secondary-button" onClick={() => onEdit(entry)} type="button">
          <Edit3 size={17} />
          Edit
        </button>
        <button className="secondary-button" onClick={() => onContinue(entry)} type="button">
          <FileText size={17} />
          Continue
        </button>
        <button className="secondary-button" onClick={() => onExport(entry, "markdown")} type="button">
          <Download size={17} />
          MD
        </button>
        <button className="secondary-button" onClick={() => onExport(entry, "json")} type="button">
          JSON
        </button>
        <button
          className="icon-button icon-button--danger"
          disabled={mutating}
          onClick={() => onDelete(entry)}
          title="Delete entry"
          type="button"
        >
          <Trash2 size={17} />
        </button>
      </div>

      <div className="tag-row">
        {entry.moodInfo.label && <span className="mood-chip">{entry.moodInfo.label}</span>}
        {entry.tags.map((tag) => (
          <span className="tag-chip" key={tag.id}>
            {tag.name}
          </span>
        ))}
      </div>

      <article className="entry-body">{entry.textPlain || entry.text}</article>

      <dl className="detail-list detail-list--compact">
        <Detail label="Number" value={formatEntryNumber(entry.id)} />
        <Detail label="UUID" value={entry.uuid} />
        <Detail label="Format" value={entry.contentFormat} />
        <Detail label="Updated" value={formatDateTime(entry.updatedAt)} />
        <Detail label="Images" value={entry.attachmentCount} />
      </dl>

      {entry.summary && (
        <div className="metadata-block">
          <h4>Summary</h4>
          <p>{entry.summary}</p>
        </div>
      )}

      {entry.location && (
        <div className="metadata-block">
          <h4>
            <MapPin size={16} />
            Location
          </h4>
          <p>{entry.location.placeName ?? `${entry.location.latitude}, ${entry.location.longitude}`}</p>
          {(entry.location.weatherCondition || weatherTemperature) && (
            <p>
              {entry.location.weatherCondition ?? "Weather"} /{" "}
              {weatherTemperature ?? "No temperature"}
            </p>
          )}
          {(entry.location.weatherHumidity !== null || entry.location.weatherWindKph !== null) && (
            <p>
              {entry.location.weatherHumidity !== null ? `Humidity ${entry.location.weatherHumidity}%` : "Humidity n/a"}
              {" / "}
              {entry.location.weatherWindKph !== null
                ? `Wind ${entry.location.weatherWindKph.toFixed(1)} kph`
                : "Wind n/a"}
            </p>
          )}
          {(entry.location.weatherIcon || entry.location.weatherFetchedAt || entry.location.source) && (
            <p>
              {[entry.location.weatherIcon, entry.location.weatherFetchedAt, entry.location.source]
                .filter(Boolean)
                .join(" / ")}
            </p>
          )}
        </div>
      )}

      {entry.thread && (
        <div className="metadata-block">
          <h4>Thread</h4>
          <p>{entry.thread.title ?? entry.thread.rootUuid}</p>
          {entry.thread.summary && <p>{entry.thread.summary}</p>}
          <p>
            {entry.thread.entryCount} entries / {entry.thread.isRoot ? "Root" : "Continuation"}
          </p>
        </div>
      )}

      <div className="metadata-block">
        <div className="metadata-heading-row">
          <h4>
            <History size={16} />
            History
          </h4>
          <button
            className="secondary-button secondary-button--small"
            disabled={historyLoading}
            onClick={() => onLoadHistory(entry)}
            type="button"
          >
            {historyLoading ? "Loading" : "Load"}
          </button>
        </div>
        {entryHistory?.entryId === entry.id ? (
          entryHistory.history.length > 0 ? (
            <div className="history-list">
              {entryHistory.history.map((item) => (
                <article className="history-row" key={item.id}>
                  <div>
                    <h5>{item.operationType.replace("EDIT_", "").toLowerCase()}</h5>
                    <p>{formatDateTime(item.timestamp)}</p>
                  </div>
                  <span>{item.changedFields.join(", ") || "metadata"}</span>
                </article>
              ))}
            </div>
          ) : (
            <p>No edit history for this entry.</p>
          )
        ) : (
          <p>Version snapshots appear here after loading.</p>
        )}
      </div>
    </Wrapper>
  );
}

type EntryStackProps = {
  entries: Entry[];
  loading: boolean;
  emptyText?: string;
};

export function EntryStack({ entries, loading, emptyText = "No entries found." }: EntryStackProps) {
  if (loading) {
    return <SkeletonList compact />;
  }

  if (entries.length === 0) {
    return <div className="empty-state">{emptyText}</div>;
  }

  return (
    <div className="entry-stack">
      {entries.map((entry) => (
        <EntryMini entry={entry} key={entry.uuid} />
      ))}
    </div>
  );
}

export function EntryMini({ entry }: { entry: Entry }) {
  return (
    <article className="entry-mini">
      <div className="entry-mini-heading">
        <h4>{entry.title || entry.textPlain.slice(0, 82) || "Untitled entry"}</h4>
        <EntryNumber entry={entry} />
      </div>
      <p>{entry.textPlain.slice(0, 140)}</p>
      <EntryMeta entry={entry} />
    </article>
  );
}

export function EntryCardContent({ entry }: { entry: Entry }) {
  return (
    <>
      <div className="entry-card-heading">
        <div>
          <p className="eyebrow">{formatDateTime(entry.createdAt)}</p>
          <h4>{entry.title || entry.textPlain.slice(0, 84) || "Untitled entry"}</h4>
        </div>
        <div className="entry-card-side">
          <EntryNumber entry={entry} />
          {entry.attachmentCount > 0 && (
            <span className="icon-stat" title="Image attachments">
              <ImageIcon size={15} />
              {entry.attachmentCount}
            </span>
          )}
        </div>
      </div>
      <p>{entry.summary || entry.textPlain.slice(0, 180)}</p>
      <EntryMeta entry={entry} />
    </>
  );
}

export function EntryNumber({ entry }: { entry: Entry }) {
  return (
    <span className="entry-number" title="Entry number">
      {formatEntryNumber(entry.id)}
    </span>
  );
}

export function EntryMeta({ entry }: { entry: Entry }) {
  const weatherTemperature = entry.location ? formatWeatherTemperature(entry.location) : null;

  return (
    <div className="entry-meta">
      {entry.moodInfo.label && <span className="mood-chip">{entry.moodInfo.label}</span>}
      {entry.tags.slice(0, 4).map((tag) => (
        <span className="tag-chip" key={tag.id}>
          <Tags size={12} />
          {tag.name}
        </span>
      ))}
      {entry.location && (
        <span className="tag-chip">
          <MapPin size={12} />
          {entry.location.placeName ?? "Location"}
        </span>
      )}
      {entry.location?.weatherCondition && (
        <span className="tag-chip">
          {weatherTemperature
            ? `${entry.location.weatherCondition}, ${weatherTemperature}`
            : entry.location.weatherCondition}
        </span>
      )}
      {entry.thread && <span className="tag-chip">{entry.thread.entryCount} in thread</span>}
    </div>
  );
}

export function DeleteEntryDialog({
  deleting,
  entry,
  onCancel,
  onConfirm,
}: {
  deleting: boolean;
  entry: Entry;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  return (
    <div className="dialog-backdrop" role="presentation">
      <section className="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="delete-entry-title">
        <div className="confirm-dialog-header">
          <div className="danger-mark" aria-hidden="true">
            <TriangleAlert size={22} />
          </div>
          <div>
            <p className="eyebrow">Delete entry</p>
            <h3 id="delete-entry-title">{entry.title || entry.textPlain.slice(0, 72) || entry.uuid}</h3>
          </div>
          <button
            className="icon-button icon-button--small"
            disabled={deleting}
            onClick={onCancel}
            title="Cancel"
            type="button"
          >
            <X size={16} />
          </button>
        </div>

        <p>
          This permanently deletes the entry after creating a verified backup.
          Later entry IDs will be resequenced.
        </p>
        <blockquote>{entry.textPlain.slice(0, 180) || "Untitled entry"}</blockquote>

        <div className="confirm-dialog-actions">
          <button className="secondary-button" disabled={deleting} onClick={onCancel} type="button">
            Cancel
          </button>
          <button className="danger-button" disabled={deleting} onClick={onConfirm} type="button">
            <Trash2 size={17} />
            {deleting ? "Deleting" : "Yes, I want to delete"}
          </button>
        </div>
      </section>
    </div>
  );
}
