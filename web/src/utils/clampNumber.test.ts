import { describe, it, expect } from "vitest";
import { clampNumberValue } from "./clampNumber";

describe("clampNumberValue", () => {
  it("returns empty string unchanged", () => {
    expect(clampNumberValue("")).toBe("");
  });

  it("returns non-numeric input unchanged", () => {
    expect(clampNumberValue("abc")).toBe("abc");
    expect(clampNumberValue("12.5x")).toBe("12.5x");
  });

  it("returns numeric input unchanged when within bounds", () => {
    expect(clampNumberValue("5", 1, 10)).toBe("5");
    expect(clampNumberValue("1", 1, 10)).toBe("1");
    expect(clampNumberValue("10", 1, 10)).toBe("10");
  });

  it("clamps to min when below", () => {
    expect(clampNumberValue("0", 1, 10)).toBe("1");
    expect(clampNumberValue("-100", 1, 10)).toBe("1");
  });

  it("clamps to max when above", () => {
    expect(clampNumberValue("11", 1, 10)).toBe("10");
    expect(clampNumberValue("999999", 1, 10)).toBe("10");
  });

  it("regression : protege contre 86400 quand max=168", () => {
    // Le bug analytics-worker : daily_snapshot_interval=86400 (heures, lu
    // comme tel) -> 9.86 ans entre 2 runs. Avec max=168 (1 semaine), on
    // ramene a 168.
    expect(clampNumberValue("86400", 1, 168)).toBe("168");
  });

  it("only min defined : clamps below, leaves above", () => {
    expect(clampNumberValue("0", 5)).toBe("5");
    expect(clampNumberValue("999", 5)).toBe("999");
  });

  it("only max defined : clamps above, leaves below", () => {
    expect(clampNumberValue("999", undefined, 100)).toBe("100");
    expect(clampNumberValue("-50", undefined, 100)).toBe("-50");
  });

  it("no bounds : returns numeric input as string", () => {
    expect(clampNumberValue("42")).toBe("42");
    expect(clampNumberValue("-1")).toBe("-1");
  });

  it("handles decimal numbers", () => {
    expect(clampNumberValue("3.14", 0, 10)).toBe("3.14");
    expect(clampNumberValue("15.5", 0, 10)).toBe("10");
  });
});
