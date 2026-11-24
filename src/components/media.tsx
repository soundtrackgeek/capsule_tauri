import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { FileImage, Images, X } from "lucide-react";
import {
  getCoverDataUrl,
  getImageDataUrl,
  getLocalImagePreviewDataUrl,
} from "../backend";
import type { ImageAttachment, ImageVariant } from "../types";

type DataUrlImageProps = {
  attachment: ImageAttachment;
  variant: ImageVariant;
  className: string;
};

export function LocalImagePreview({ filePath, altText }: { filePath: string; altText: string }) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const normalized = filePath.trim();
    setSrc(null);
    setFailed(false);
    if (!normalized) {
      return () => {
        cancelled = true;
      };
    }

    getLocalImagePreviewDataUrl(normalized)
      .then((dataUrl) => {
        if (!cancelled) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [filePath]);

  if (failed) {
    return (
      <div className="composer-preview-thumb composer-preview-placeholder">
        <FileImage size={20} />
      </div>
    );
  }

  if (!src) {
    return <div className="composer-preview-thumb composer-preview-placeholder skeleton" />;
  }

  return <img alt={altText || "Queued image preview"} className="composer-preview-thumb" src={src} />;
}

export function DataUrlImage({ attachment, variant, className }: DataUrlImageProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setSrc(null);
    setFailed(false);
    getImageDataUrl(attachment.attachmentId, variant)
      .then((dataUrl) => {
        if (!cancelled) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [attachment.attachmentId, variant]);

  if (failed) {
    return (
      <div className={`${className} media-placeholder`}>
        <FileImage size={24} />
      </div>
    );
  }

  if (!src) {
    return <div className={`${className} media-placeholder skeleton`} />;
  }

  return <img alt={attachment.altText ?? attachment.caption ?? "Entry image"} className={className} src={src} />;
}

type CoverImageProps = {
  filename: string;
  variant: ImageVariant;
  className: string;
};

export function CoverImage({ filename, variant, className }: CoverImageProps) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setSrc(null);
    setFailed(false);
    getCoverDataUrl(filename, variant)
      .then((dataUrl) => {
        if (!cancelled) {
          setSrc(dataUrl);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFailed(true);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [filename, variant]);

  if (failed) {
    return (
      <div className={`${className} media-placeholder`}>
        <Images size={24} />
      </div>
    );
  }

  if (!src) {
    return <div className={`${className} media-placeholder skeleton`} />;
  }

  return <img alt={filename} className={className} src={src} />;
}

export function ImageLightbox({
  attachment,
  onClose,
}: {
  attachment: ImageAttachment;
  onClose: () => void;
}) {
  const titleId = useId();
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useLayoutEffect(() => {
    const trigger = document.activeElement;
    closeButtonRef.current?.focus();
    return () => {
      if (trigger instanceof HTMLElement && trigger.isConnected) {
        trigger.focus();
      }
    };
  }, []);

  return (
    <div
      aria-labelledby={titleId}
      aria-modal="true"
      className="lightbox"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
        } else if (event.key === "Tab") {
          event.preventDefault();
        }
      }}
      role="dialog"
    >
      <div className="lightbox-toolbar">
        <div>
          <p className="eyebrow">{attachment.mimeType}</p>
          <h3 id={titleId}>{attachment.caption || attachment.altText || attachment.hash}</h3>
        </div>
        <button className="icon-button" onClick={onClose} ref={closeButtonRef} title="Close" type="button">
          <X size={18} />
        </button>
      </div>
      <DataUrlImage
        attachment={attachment}
        className="lightbox-image"
        variant={attachment.originalAvailable ? "full" : "thumb"}
      />
    </div>
  );
}
