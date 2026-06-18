import { describe, expect, it } from "vitest";
import { ru } from "../i18n/ru";
import { interpolate, resolvePath } from "../i18n/translate";
import {
  createHeheButtonLabel,
  defaultHeheNameFromPaths,
  resolveCreateSources,
} from "./createHeheSources";

const t = (key: string, vars?: Record<string, string | number>) =>
  interpolate(resolvePath(ru, key) ?? key, vars);
import type { ArchiveEntry } from "../types";

const file = (path: string): ArchiveEntry => ({
  path,
  name: path.split("/").pop() ?? path,
  size: 10,
  packedSize: 5,
  isDir: false,
  extension: "stl",
});

describe("resolveCreateSources", () => {
  it("prefers clipboard over archive context", () => {
    const resolved = resolveCreateSources({
      clipboardPaths: ["C:/clip/folder"],
      archivePath: "C:/open.hehe",
      selected: new Set(["a.stl"]),
      allEntries: [file("a.stl")],
      currentFolder: "",
    });
    expect(resolved?.kind).toBe("local");
    if (resolved?.kind === "local") {
      expect(resolved.paths).toEqual(["C:/clip/folder"]);
    }
  });

  it("uses archive selection when clipboard empty", () => {
    const resolved = resolveCreateSources({
      clipboardPaths: [],
      archivePath: "C:/open.hehe",
      selected: new Set(["part.stl"]),
      allEntries: [file("part.stl"), file("other.stl")],
      currentFolder: "",
    });
    expect(resolved).toEqual({
      kind: "archive",
      archivePath: "C:/open.hehe",
      entryPaths: ["part.stl"],
      stripPrefix: null,
      sourceName: "open",
    });
  });

  it("packs current folder with strip prefix", () => {
    const resolved = resolveCreateSources({
      clipboardPaths: [],
      archivePath: "C:/open.hehe",
      selected: new Set(),
      allEntries: [file("refs/a.png"), file("refs/b.png"), file("root.stl")],
      currentFolder: "refs",
    });
    expect(resolved?.kind).toBe("archive");
    if (resolved?.kind === "archive") {
      expect(resolved.stripPrefix).toBe("refs");
      expect(resolved.entryPaths).toEqual(["refs/a.png", "refs/b.png"]);
      expect(resolved.sourceName).toBe("refs");
    }
  });

  it("returns null when no clipboard and no archive", () => {
    expect(
      resolveCreateSources({
        clipboardPaths: [],
        archivePath: null,
        selected: new Set(),
        allEntries: [],
        currentFolder: "",
      }),
    ).toBeNull();
  });
});

describe("createHeheButtonLabel", () => {
  it("shows contextual labels", () => {
    expect(
      createHeheButtonLabel(
        {
          archivePath: "x.hehe",
          selectedCount: 2,
          currentFolder: "",
        },
        t,
      ),
    ).toBe(t("toolbar.createHeheFromSelection"));
    expect(
      createHeheButtonLabel(
        {
          archivePath: "x.hehe",
          selectedCount: 0,
          currentFolder: "refs",
        },
        t,
      ),
    ).toBe(t("toolbar.createHeheFromFolder"));
    expect(
      createHeheButtonLabel(
        {
          archivePath: null,
          selectedCount: 0,
          currentFolder: "",
        },
        t,
      ),
    ).toBe(t("toolbar.createHehe"));
  });
});

describe("defaultHeheNameFromPaths", () => {
  it("uses folder stem for single path", () => {
    expect(defaultHeheNameFromPaths(["C:/Projects/MyPart"])).toBe("MyPart");
  });

  it("uses timestamped name for multiple paths", () => {
    const name = defaultHeheNameFromPaths(["a", "b"]);
    expect(name.startsWith("Archive-")).toBe(true);
  });
});
