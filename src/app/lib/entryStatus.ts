import {
  STATUS_LABEL_SENT_TO_PRINT,
  STATUS_SORT_ORDER_SENT_TO_PRINT,
} from "./workflowConstants";
import type { EntryStatusMap, WorkflowStatus } from "../types";

export function pathsWithoutStatus(
  entryPaths: string[],
  statusMap: EntryStatusMap,
): string[] {
  return entryPaths.filter((p) => !statusMap[p]);
}

export function findPrintStatusId(
  statuses: WorkflowStatus[],
): string | undefined {
  return (
    statuses.find((s) => s.label === STATUS_LABEL_SENT_TO_PRINT)?.id ??
    statuses.find((s) => s.sortOrder === STATUS_SORT_ORDER_SENT_TO_PRINT)?.id
  );
}
