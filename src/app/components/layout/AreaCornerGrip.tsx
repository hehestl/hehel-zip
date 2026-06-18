import { useRef, useState } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useI18n } from "../../i18n";
import { closeCurrentWindow } from "../../lib/windowManager";
import { AreaCloseArrowOverlay, type CloseOverlayRef } from "./AreaCloseArrowOverlay";
import { AreaGripContextMenu } from "./AreaGripContextMenu";

interface Props {
  leafId: string;
  onSplit: (leafId: string, direction: "row" | "col") => void;
  canPopOut: boolean;
  canClosePanel: boolean;
  onPopOut: (leafId: string) => void;
  onClosePanel: (leafId: string) => void;
}

type GestureState = "idle" | "splitPending" | "closePending";

const SPLIT_DISTANCE = 8;
const CLOSE_DISTANCE = 24;

export function AreaCornerGrip({
  leafId,
  onSplit,
  canPopOut,
  canClosePanel,
  onPopOut,
  onClosePanel,
}: Props) {
  const { t } = useI18n();
  const overlayRef = useRef<CloseOverlayRef>(null);
  const startPos = useRef<{ x: number; y: number } | null>(null);
  const gestureState = useRef<GestureState>("idle");
  const [menuCoords, setMenuCoords] = useState<{ x: number; y: number } | null>(
    null,
  );

  const isMainWindow = getCurrentWebviewWindow().label === "main";

  const cleanupGesture = (e: React.PointerEvent<HTMLDivElement>) => {
    try {
      e.currentTarget.releasePointerCapture(e.pointerId);
    } catch {
      // pointer already released
    }
    startPos.current = null;
    gestureState.current = "idle";
    overlayRef.current?.update(0, 0, false, false);
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setMenuCoords({ x: e.clientX, y: e.clientY });
  };

  return (
    <>
      <div
        role="button"
        tabIndex={0}
        className="h-5 w-5 shrink-0 cursor-se-resize border border-hh-border bg-hh-surface transition-colors hover:bg-hh-accent/20 active:bg-hh-accent/40"
        style={{ clipPath: "polygon(100% 0, 0 100%, 100% 100%)" }}
        title={t("layout.gripTitle")}
        aria-label={t("layout.gripAria")}
        onContextMenu={handleContextMenu}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          startPos.current = { x: e.clientX, y: e.clientY };
          gestureState.current = e.shiftKey ? "closePending" : "splitPending";
        }}
        onPointerMove={(e) => {
          if (!startPos.current) return;

          const dx = e.clientX - startPos.current.x;
          const dy = e.clientY - startPos.current.y;
          const distance = Math.hypot(dx, dy);

          if (gestureState.current === "splitPending" && distance > SPLIT_DISTANCE) {
            const direction = Math.abs(dx) > Math.abs(dy) ? "row" : "col";
            onSplit(leafId, direction);
            cleanupGesture(e);
            return;
          }

          if (gestureState.current === "closePending") {
            overlayRef.current?.update(dx, dy, true, isMainWindow);
          }
        }}
        onPointerUp={(e) => {
          if (!startPos.current) return;

          const dx = e.clientX - startPos.current.x;
          const dy = e.clientY - startPos.current.y;
          const distance = Math.hypot(dx, dy);

          if (
            gestureState.current === "closePending" &&
            distance > CLOSE_DISTANCE &&
            !isMainWindow
          ) {
            void closeCurrentWindow();
          }

          cleanupGesture(e);
        }}
        onPointerCancel={cleanupGesture}
      />
      {menuCoords ? (
        <AreaGripContextMenu
          x={menuCoords.x}
          y={menuCoords.y}
          canPopOut={canPopOut}
          canClosePanel={canClosePanel}
          canCloseWindow={!isMainWindow}
          onPopOut={() => onPopOut(leafId)}
          onClosePanel={() => onClosePanel(leafId)}
          onCloseWindow={() => void closeCurrentWindow()}
          onClose={() => setMenuCoords(null)}
        />
      ) : null}
      <AreaCloseArrowOverlay ref={overlayRef} />
    </>
  );
}