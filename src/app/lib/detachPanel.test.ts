import { beforeEach, describe, expect, it, vi } from "vitest";
import { consumeDetachPayload, stashDetachPayload } from "./detachPanel";

function createLocalStorageMock(): Storage {
  const store = new Map<string, string>();
  return {
    get length() {
      return store.size;
    },
    clear: () => store.clear(),
    getItem: (key) => store.get(key) ?? null,
    key: (index) => [...store.keys()][index] ?? null,
    removeItem: (key) => {
      store.delete(key);
    },
    setItem: (key, value) => {
      store.set(key, value);
    },
  };
}

describe("detachPanel", () => {
  beforeEach(() => {
    vi.stubGlobal("localStorage", createLocalStorageMock());
  });

  it("stash and consume round-trip", () => {
    const id = stashDetachPayload({
      archivePath: "C:\\a.hehe",
      mode: "images",
      title: "a.hehe",
    });
    const payload = consumeDetachPayload(id);
    expect(payload).toEqual({
      archivePath: "C:\\a.hehe",
      mode: "images",
      title: "a.hehe",
    });
  });

  it("consume removes key one-shot", () => {
    const id = stashDetachPayload({
      archivePath: "x.zip",
      mode: "archive",
      title: "x.zip",
    });
    consumeDetachPayload(id);
    expect(consumeDetachPayload(id)).toBeNull();
  });
});
