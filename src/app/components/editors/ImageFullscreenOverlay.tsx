import { useEffect } from "react";
import { useI18n } from "../../i18n";

interface Props {
  open: boolean;
  src: string | null;
  name: string;
  index: number;
  total: number;
  onClose: () => void;
  onPrev: () => void;
  onNext: () => void;
  onImageError: () => void;
}

export function ImageFullscreenOverlay({
  open,
  src,
  name,
  index,
  total,
  onClose,
  onPrev,
  onNext,
  onImageError,
}: Props) {
  const { t } = useI18n();

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      if (e.key === "ArrowLeft") onPrev();
      if (e.key === "ArrowRight") onNext();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose, onNext, onPrev]);

  if (!open) return null;

  return (
    <div
      className="image-fullscreen"
      role="dialog"
      aria-modal
      onClick={onClose}
    >
      <div
        className="image-fullscreen__inner"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="image-fullscreen__bar">
          <button type="button" className="btn btn-ghost" onClick={onPrev}>
            ←
          </button>
          <span className="text-xs">
            {t("images.fullscreenCounter", {
              current: index + 1,
              total,
              name,
            })}
          </span>
          <button type="button" className="btn btn-ghost" onClick={onNext}>
            →
          </button>
          <button
            type="button"
            className="btn btn-ghost ml-auto"
            onClick={onClose}
          >
            {t("images.close")}
          </button>
        </div>
        {src ? (
          <img
            src={src}
            alt={name}
            className="image-fullscreen__img"
            onError={onImageError}
          />
        ) : null}
      </div>
    </div>
  );
}