import { beforeEach, describe, expect, it, vi } from "vitest";
import { fetchAllArchiveEntries } from "./fetchArchiveEntries";
import type { ArchiveEntry } from "../types";

vi.mock("../api", () => ({
  api: {
    listArchiveEntriesPaginated: vi.fn(),
  },
}));

import { api } from "../api";

const entry = (path: string): ArchiveEntry => ({
  path,
  name: path,
  size: 1,
  packedSize: 1,
  isDir: false,
  extension: "stl",
});

describe("fetchAllArchiveEntries", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns first page when total fits one page", async () => {
    vi.mocked(api.listArchiveEntriesPaginated).mockResolvedValueOnce({
      entries: [entry("a.stl")],
      totalCount: 1,
      offset: 0,
      limit: 5000,
    });
    const result = await fetchAllArchiveEntries("C:\\a.zip");
    expect(result).toHaveLength(1);
    expect(api.listArchiveEntriesPaginated).toHaveBeenCalledTimes(1);
  });

  it("fetches remaining pages", async () => {
    const firstPage = {
      entries: [entry("a.stl")],
      totalCount: 3,
      offset: 0,
      limit: 1,
    };
    vi.mocked(api.listArchiveEntriesPaginated)
      .mockResolvedValueOnce({
        entries: [entry("b.stl")],
        totalCount: 3,
        offset: 1,
        limit: 1,
      })
      .mockResolvedValueOnce({
        entries: [entry("c.stl")],
        totalCount: 3,
        offset: 2,
        limit: 1,
      });

    const result = await fetchAllArchiveEntries("C:\\a.zip", 1, firstPage);
    expect(result.map((e) => e.path)).toEqual(["a.stl", "b.stl", "c.stl"]);
    expect(api.listArchiveEntriesPaginated).toHaveBeenCalledTimes(2);
  });
});