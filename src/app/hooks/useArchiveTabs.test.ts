import { describe, expect, it } from "vitest";
import { ru } from "../i18n/ru";
import { interpolate, resolvePath } from "../i18n/translate";
import {
  archiveTabTitle,
  createEmptyTabMetadata,
  createTabWithPath,
} from "../lib/archiveTabs";

const t = (key: string, vars?: Record<string, string | number>) =>
  interpolate(resolvePath(ru, key) ?? key, vars);

describe("archiveTabs helpers", () => {
  it("createEmptyTabMetadata", () => {
    const tab = createEmptyTabMetadata(t);
    expect(tab.title).toBe(t("tabs.newTab"));
    expect(tab.archivePath).toBeNull();
    expect(tab.initialPath).toBeUndefined();
    expect(tab.id).toMatch(/^tab-/);
  });

  it("createTabWithPath sets initialPath and basename title", () => {
    const tab = createTabWithPath("C:\\parts\\job.zip", t);
    expect(tab.initialPath).toBe("C:\\parts\\job.zip");
    expect(tab.title).toBe("job.zip");
    expect(tab.archivePath).toBeNull();
  });

  it("archiveTabTitle returns basename or empty label", () => {
    expect(archiveTabTitle(null, t)).toBe(t("tabs.newTab"));
    expect(archiveTabTitle("/foo/bar.7z", t)).toBe("bar.7z");
  });

  it("openPathInNewTab pattern preserves existing tab ids", () => {
    const first = createEmptyTabMetadata(t);
    const second = createTabWithPath("a.zip", t);
    const tabs = [first, second];
    expect(tabs).toHaveLength(2);
    expect(tabs[0].id).not.toBe(tabs[1].id);
  });
});