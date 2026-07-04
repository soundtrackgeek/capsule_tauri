import { useEffect, useState } from "react";
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
  return (
    <div className="lightbox" role="dialog" aria-modal="true">
      <div className="lightbox-toolbar">
        <div>
          <p className="eyebrow">{attachment.mimeType}</p>
          <h3>{attachment.caption || attachment.altText || attachment.hash}</h3>
        </div>
        <button className="icon-button" onClick={onClose} title="Close" type="button">
          <X size={18} />
        </button>
      </div>
      <DataUrlImage attachment={attachment} className="lightbox-image" variant="full" />
    </div>
  );
}
