import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { WorkflowStatus } from "../types";

export function useWorkflowStatuses() {
  const [statuses, setStatuses] = useState<WorkflowStatus[]>([]);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      setStatuses(await api.getWorkflowStatuses());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  return { statuses, loading, reload };
}
