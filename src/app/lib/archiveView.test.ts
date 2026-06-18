import { describe, expect, it } from "vitest";
import {
  countFilesInFolder,
  countImagesInFolder,
  countStlObjInFolder,
  folderStatsFromVisible,
  formatBytes,
  formatLocationBar,
  folderExists,
  getVisibleEntries,
  parentFolder,
  parseLocationInput,
} from "./archiveView";
import type { ArchiveEntry } from "../types";

describe("archiveView", () => {
  it("formatBytes", () => {
    expect(formatBytes(1024)).toContain("KB");
  });

  it("getVisibleEntries root", () => {
    const entries: ArchiveEntry[] = [
      {
        path: "folder/",
        name: "folder",
        size: 0,
        packedSize: 0,
        isDir: true,
        extension: "",
      },
      {
        path: "folder/part.stl",
        name: "part.stl",
        size: 100,
        packedSize: 50,
        isDir: false,
        extension: "stl",
      },
    ];
    const visible = getVisibleEntries(entries, "");
    expect(visible.some((e) => e.name === "folder")).toBe(true);
  });

  it("parentFolder", () => {
    expect(parentFolder("")).toBe("");
    expect(parentFolder("models")).toBe("");
    expect(parentFolder("models/parts")).toBe("models");
    expect(parentFolder("models\\parts\\")).toBe("models");
  });

  it("folderExists", () => {
    const entries: ArchiveEntry[] = [
      {
        path: "folder/",
        name: "folder",
        size: 0,
        packedSize: 0,
        isDir: true,
        extension: "",
      },
      {
        path: "folder/part.stl",
        name: "part.stl",
        size: 100,
        packedSize: 50,
        isDir: false,
        extension: "stl",
      },
    ];
    expect(folderExists(entries, "")).toBe(true);
    expect(folderExists(entries, "folder")).toBe(true);
    expect(folderExists(entries, "missing")).toBe(false);
  });

  it("formatLocationBar", () => {
    expect(formatLocationBar("C:\\archives\\job.zip", "")).toBe("job.zip");
    expect(formatLocationBar("C:/archives/job.zip", "models/parts")).toBe(
      "job.zip\\models\\parts",
    );
  });

  it("parseLocationInput", () => {
    expect(parseLocationInput("job.zip", "job.zip")).toBe("");
    expect(parseLocationInput("job.zip\\models", "job.zip")).toBe("models");
    expect(parseLocationInput("models/parts/", "job.zip")).toBe("models/parts");
  });

  it("folderStatsFromVisible counts in one pass", () => {
    const visible: ArchiveEntry[] = [
      {
        path: "photo.png",
        name: "photo.png",
        size: 10,
        packedSize: 5,
        isDir: false,
        extension: "png",
      },
      {
        path: "part.stl",
        name: "part.stl",
        size: 100,
        packedSize: 50,
        isDir: false,
        extension: "stl",
      },
      {
        path: "nested/",
        name: "nested",
        size: 0,
        packedSize: 0,
        isDir: true,
        extension: "",
      },
    ];
    expect(folderStatsFromVisible(visible)).toEqual({
      images: 1,
      models: 1,
      files: 2,
    });
  });

  it("folder counts hide zero entries", () => {
    const entries: ArchiveEntry[] = [
      {
        path: "photo.png",
        name: "photo.png",
        size: 10,
        packedSize: 5,
        isDir: false,
        extension: "png",
      },
      {
        path: "part.stl",
        name: "part.stl",
        size: 100,
        packedSize: 50,
        isDir: false,
        extension: "stl",
      },
    ];
    expect(countImagesInFolder(entries, "")).toBe(1);
    expect(countStlObjInFolder(entries, "")).toBe(1);
    expect(countFilesInFolder(entries, "")).toBe(2);
  });
});
