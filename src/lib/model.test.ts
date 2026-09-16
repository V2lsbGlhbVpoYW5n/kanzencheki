import { describe, expect, it } from "vitest";
import { coverPhoto, type Cheki } from "./model";
const cheki = {
  id: "same-name",
  people: ["A"],
  shotType: "solo",
  assets: [
    { id: "scan", src: "scan.jpg", crop: { x: 0.2, y: 0.1, w: 0.3, h: 0.5 } },
  ],
  coverAssetId: "scan",
} as Cheki;
describe("shared collection previews", () => {
  it("uses the selected scan crop for browser thumbnails", () => {
    expect(coverPhoto(cheki, false).crop).toEqual({
      x: 20,
      y: 10,
      w: 30,
      h: 50,
    });
  });
  it("does not crop a native preview twice", () => {
    expect(coverPhoto(cheki, true)).toEqual({
      src: "scan.jpg",
      title: "A",
      crop: undefined,
    });
  });
});
