import type { EditorMode } from "../../types";
import { clampRatio } from "../../lib/areaLayout";
import type { AreaNode } from "../../types";
import { AreaCornerGrip } from "./AreaCornerGrip";
import { AreaHeader } from "./AreaHeader";
import { AreaSplitDivider } from "./AreaSplitDivider";

interface Props {
  node: AreaNode;
  layout: AreaNode;
  focusedLeafId: string;
  disabledReasons: Partial<Record<EditorMode, string>>;
  onFocusLeaf: (id: string) => void;
  onSplit: (leafId: string, direction: "row" | "col") => void;
  onResize: (splitId: string, ratio: number) => void;
  onModeChange: (leafId: string, mode: EditorMode) => void;
  canPopOut: boolean;
  canClosePanel: boolean;
  onPopOutPanel: (leafId: string) => void;
  onClosePanel: (leafId: string) => void;
  renderEditor: (mode: EditorMode, leafId: string) => React.ReactNode;
}

export function AreaLayoutRoot({
  node,
  layout,
  focusedLeafId,
  disabledReasons,
  onFocusLeaf,
  onSplit,
  onResize,
  onModeChange,
  canPopOut,
  canClosePanel,
  onPopOutPanel,
  onClosePanel,
  renderEditor,
}: Props) {
  if (node.kind === "leaf") {
    return (
      <div
        className={`flex min-h-0 min-w-0 flex-1 flex-col ${
          node.id === focusedLeafId ? "ring-1 ring-hh-accent" : ""
        }`}
        onMouseDown={() => onFocusLeaf(node.id)}
      >
        <AreaHeader
          mode={node.mode}
          disabledReasons={disabledReasons}
          onModeChange={(mode) => onModeChange(node.id, mode)}
          onFocus={() => onFocusLeaf(node.id)}
        >
          <AreaCornerGrip
            leafId={node.id}
            onSplit={onSplit}
            canPopOut={canPopOut}
            canClosePanel={canClosePanel}
            onPopOut={onPopOutPanel}
            onClosePanel={onClosePanel}
          />
        </AreaHeader>
        <div className="flex min-h-0 flex-1 flex-col">{renderEditor(node.mode, node.id)}</div>
      </div>
    );
  }

  const flexDir = node.direction === "row" ? "flex-row" : "flex-col";

  return (
    <div className={`flex min-h-0 min-w-0 flex-1 ${flexDir}`}>
      <div
        className="flex min-h-0 min-w-0"
        style={{
          flex: node.ratio,
        }}
      >
        <AreaLayoutRoot
          node={node.a}
          layout={layout}
          focusedLeafId={focusedLeafId}
          disabledReasons={disabledReasons}
          onFocusLeaf={onFocusLeaf}
          onSplit={onSplit}
          onResize={onResize}
          onModeChange={onModeChange}
          canPopOut={canPopOut}
          canClosePanel={canClosePanel}
          onPopOutPanel={onPopOutPanel}
          onClosePanel={onClosePanel}
          renderEditor={renderEditor}
        />
      </div>
      <AreaSplitDivider
        direction={node.direction}
        onDrag={(delta, size) => {
          const next = clampRatio(node.ratio + delta / size);
          onResize(node.id, next);
        }}
      />
      <div
        className="flex min-h-0 min-w-0"
        style={{
          flex: 1 - node.ratio,
        }}
      >
        <AreaLayoutRoot
          node={node.b}
          layout={layout}
          focusedLeafId={focusedLeafId}
          disabledReasons={disabledReasons}
          onFocusLeaf={onFocusLeaf}
          onSplit={onSplit}
          onResize={onResize}
          onModeChange={onModeChange}
          canPopOut={canPopOut}
          canClosePanel={canClosePanel}
          onPopOutPanel={onPopOutPanel}
          onClosePanel={onClosePanel}
          renderEditor={renderEditor}
        />
      </div>
    </div>
  );
}
