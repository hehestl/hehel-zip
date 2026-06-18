import { describe, expect, it } from "vitest";
import { findPrintStatusId, pathsWithoutStatus } from "./entryStatus";
import { STATUS_LABEL_SENT_TO_PRINT } from "./workflowConstants";
import type { EntryStatusMap, WorkflowStatus } from "../types";

describe("pathsWithoutStatus", () => {
  it("returns only paths without status", () => {
    const statusMap: EntryStatusMap = { "a.stl": "id1" };
    expect(pathsWithoutStatus(["a.stl", "b.stl", "c.stl"], statusMap)).toEqual([
      "b.stl",
      "c.stl",
    ]);
  });

  it("returns empty when all have status", () => {
    const statusMap: EntryStatusMap = { "a.stl": "id1" };
    expect(pathsWithoutStatus(["a.stl"], statusMap)).toEqual([]);
  });
});

describe("findPrintStatusId", () => {
  const statuses: WorkflowStatus[] = [
    {
      id: "pre",
      label: "Предпродакшен",
      color: "#000",
      sortOrder: 0,
      isDefault: true,
    },
    {
      id: "print",
      label: STATUS_LABEL_SENT_TO_PRINT,
      color: "#3b82f6",
      sortOrder: 1,
      isDefault: true,
    },
  ];

  it("finds by canonical label", () => {
    expect(findPrintStatusId(statuses)).toBe("print");
  });

  it("falls back to sortOrder when label renamed", () => {
    const renamed = statuses.map((s) =>
      s.id === "print" ? { ...s, label: "В печать" } : s,
    );
    expect(findPrintStatusId(renamed)).toBe("print");
  });
});
