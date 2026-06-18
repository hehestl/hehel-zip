import { describe, expect, it, vi, beforeEach } from "vitest";

const { extractToSession, dropExtractSession, startFileDrag } = vi.hoisted(
  () => ({
    extractToSession: vi.fn(),
    dropExtractSession: vi.fn(),
    startFileDrag: vi.fn(),
  }),
);

vi.mock("../api", () => ({
  api: {
    extractToSession,
    dropExtractSession,
    startFileDrag,
    copyFilesToClipboard: vi.fn(),
  },
}));

import { api } from "../api";

describe("dragEntries cancel flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    extractToSession.mockResolvedValue({
      sessionId: "sess-1",
      paths: ["/tmp/model.stl"],
    });
    dropExtractSession.mockResolvedValue(undefined);
    startFileDrag.mockResolvedValue(undefined);
  });

  it("drops session and skips startFileDrag when cancelRef is set", async () => {
    const cancelRef = { current: true };
    const entryPaths = ["model.stl"];

    const { sessionId, paths: extracted } = await api.extractToSession(
      "archive.hehe",
      entryPaths,
      true,
    );

    if (cancelRef.current) {
      await api.dropExtractSession(sessionId);
    } else {
      await api.startFileDrag(extracted, sessionId);
    }

    expect(dropExtractSession).toHaveBeenCalledWith("sess-1");
    expect(startFileDrag).not.toHaveBeenCalled();
  });

  it("starts native drag when cancelRef is false", async () => {
    const cancelRef = { current: false };
    const entryPaths = ["model.stl"];

    const { sessionId, paths: extracted } = await api.extractToSession(
      "archive.hehe",
      entryPaths,
      true,
    );

    if (cancelRef.current) {
      await api.dropExtractSession(sessionId);
    } else {
      await api.startFileDrag(extracted, sessionId);
    }

    expect(startFileDrag).toHaveBeenCalledWith(["/tmp/model.stl"], "sess-1");
    expect(dropExtractSession).not.toHaveBeenCalled();
  });
});
