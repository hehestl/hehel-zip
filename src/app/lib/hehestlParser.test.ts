import { describe, expect, it } from "vitest";
import { linkRowsFromRaw, parseHehestl, parseOsValue } from "./hehestlParser";

const SAMPLE_HEHESTL = `Кат: Игры | Cat: Games
Назв: Чёрная Кошка
Name: Black Cat
Источ: Марвел Соперники
Source: Marvel Rivals
Теги: #марвел #соперники #чёрнаякошка #женщина #девушка #супергероиня #хехестл
Tags: #marvel #rivals #blackcat #female #woman #superheroine #hehestl
Studio: h3LLcreator
PD:  | UD: 17.06.26
ART: CHS-GAME-FH-H3LL-MRIV-BLCAT-001-R01
BARCODE: CHE23114117062026

O/S: 1-4
O/S: 1-6
O/S: 1-7
O/S: 1-9
O/S: 1-12
🟧 Средняя (https://t.me/+IriLKBMIFRsxZDAy) | Medium (https://t.me/+IriLKBMIFRsxZDAy)
🪙Заказ (https://t.me/+IoDj5oVGOtxmMzgy) | Order (https://t.me/+IoDj5oVGOtxmMzgy)
Black Cat (https://t.me/c/2016710896/1884/1885) Marvel Rivals (https://t.me/c/2178887293/9350/9351) H3LLBLCATR01 (https://t.me/c/1667301240/3297)`;

describe("hehestlParser", () => {
  it("parses bilingual fields and tags", () => {
    const doc = parseHehestl(`ArchiveId: abc-123
Кат: Оригинальный | Cat: Original
Tags: #cyber #punk
Name: Cyber Samurai`);
    expect(doc.fields.some((f) => f.key === "Кат")).toBe(true);
    expect(doc.fields.some((f) => f.key === "Cat")).toBe(true);
    expect(doc.tags.length).toBeGreaterThanOrEqual(2);
  });

  it("parses full studio metadata sample", () => {
    const doc = parseHehestl(SAMPLE_HEHESTL);

    for (const key of ["Кат", "Cat", "Назв", "Name", "Studio", "ART", "BARCODE", "PD", "UD"]) {
      expect(doc.fields.some((f) => f.key === key), `missing field ${key}`).toBe(true);
    }

    expect(doc.fields.find((f) => f.key === "PD")?.value).toBe("");
    expect(doc.fields.find((f) => f.key === "UD")?.value).toBe("17.06.26");

    expect(doc.tags.length).toBeGreaterThanOrEqual(12);

    expect(doc.scales.map((s) => s.scale)).toEqual(["1-4", "1-6", "1-7", "1-9", "1-12"]);

    expect(doc.links.length).toBeGreaterThanOrEqual(5);
    expect(doc.links.some((l) => l.label.includes("H3LLBLCATR01"))).toBe(true);
    expect(
      doc.links.some((l) => l.label.includes("Средняя") || l.label === "Medium"),
    ).toBe(true);

    const rows = linkRowsFromRaw(doc.rawLines);
    expect(rows.length).toBeGreaterThanOrEqual(3);
    expect(rows.some((r) => r.length === 3)).toBe(true);
  });

  it("parses O/S with flexible sizes", () => {
    const doc = parseHehestl(`O/S: 1-4 (120mm)
O/S: 1-6 — 15 cm
O/S: 1-7, 180mm`);

    expect(doc.scales).toEqual([
      { scale: "1-4", size: "120mm" },
      { scale: "1-6", size: "15 cm" },
      { scale: "1-7", size: "180mm" },
    ]);
  });

  it("parseOsValue handles edge patterns", () => {
    expect(parseOsValue("1-12")).toEqual({ scale: "1-12" });
    expect(parseOsValue("1-4 (15 cm)")).toEqual({ scale: "1-4", size: "15 cm" });
    expect(parseOsValue("1-4 - 120mm")).toEqual({ scale: "1-4", size: "120mm" });
  });
});
