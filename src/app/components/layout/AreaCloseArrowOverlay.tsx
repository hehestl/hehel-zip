import { forwardRef, useImperativeHandle, useState } from "react";
import { useI18n } from "../../i18n";
import { resolveCloseDirection, type CloseDirection } from "../../lib/areaGripClose";

export type CloseOverlayRef = {
  update: (dx: number, dy: number, active: boolean, isMain: boolean) => void;
};

const ARROW_ICONS: Record<CloseDirection, string> = {
  left: "←",
  right: "→",
  up: "↑",
  down: "↓",
};

const SHOW_ARROW_DISTANCE = 8;
const TRIGGER_DISTANCE = 24;

export const AreaCloseArrowOverlay = forwardRef<CloseOverlayRef>((_, ref) => {
  const { t } = useI18n();
  const [state, setState] = useState<{
    visible: boolean;
    dir: CloseDirection | null;
    isMain: boolean;
    distance: number;
  }>({ visible: false, dir: null, isMain: false, distance: 0 });

  useImperativeHandle(ref, () => ({
    update: (dx, dy, active, isMain) => {
      if (!active) {
        setState((s) => (s.visible ? { ...s, visible: false, dir: null } : s));
        return;
      }

      const distance = Math.hypot(dx, dy);
      const dir = distance > SHOW_ARROW_DISTANCE ? resolveCloseDirection(dx, dy) : null;
      setState({ visible: true, dir, isMain, distance });
    },
  }));

  if (!state.visible) return null;

  const isTriggered = state.distance > TRIGGER_DISTANCE;

  return (
    <div className="pointer-events-none fixed inset-0 z-[9999] flex items-center justify-center bg-[rgb(0_0_0/0.2)] backdrop-blur-[1px]">
      <div
        className={`flex flex-col items-center justify-center rounded-hh-sm border p-6 transition-all duration-150 ${
          isTriggered
            ? "border-hh-accent bg-hh-bg shadow-[0_0_15px_rgb(38_89_255/0.2)]"
            : "border-hh-border bg-hh-surface"
        }`}
      >
        {state.isMain ? (
          <div className="max-w-xs text-center text-sm text-muted">
            <span className="mb-1 block font-medium text-hh-accent">
              {t("layout.mainWindowHint")}
            </span>
            {t("layout.mainWindowText")}
          </div>
        ) : (
          <div className="flex flex-col items-center gap-2">
            <div
              className={`text-3xl font-medium transition-transform duration-100 ${
                isTriggered ? "scale-125 text-hh-accent" : "text-muted"
              }`}
            >
              {state.dir ? ARROW_ICONS[state.dir] : "•"}
            </div>
            <div className="text-xs uppercase tracking-wider text-muted">
              {isTriggered ? t("layout.releaseToClose") : t("layout.pullFurther")}
            </div>
          </div>
        )}
      </div>
    </div>
  );
});

AreaCloseArrowOverlay.displayName = "AreaCloseArrowOverlay";