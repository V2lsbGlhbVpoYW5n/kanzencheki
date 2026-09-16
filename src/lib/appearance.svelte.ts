export type Appearance = "system" | "light" | "dark";
export const appearance = $state<{
  mode: Appearance;
  fit: "contain" | "cover";
}>({ mode: "system", fit: "contain" });
export function setAppearance(mode: Appearance) {
  appearance.mode = mode;
  try {
    localStorage.setItem("cheki-appearance", mode);
  } catch {
    /* Apply for this session if storage is unavailable. */
  }
  applyAppearance();
}
function applyAppearance() {
  const dark =
    appearance.mode === "dark" ||
    (appearance.mode === "system" &&
      matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "silk";
}
export function initAppearance() {
  try {
    const fit = localStorage.getItem("cheki-image-fit");
    if (fit === "cover" || fit === "contain") appearance.fit = fit;
    const saved = localStorage.getItem("cheki-appearance");
    if (saved === "dark" || saved === "light" || saved === "system")
      appearance.mode = saved;
  } catch {}
  const media = matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", applyAppearance);
  applyAppearance();
  return () => media.removeEventListener("change", applyAppearance);
}

export function setImageFit(fit: "cover" | "contain") {
  appearance.fit = fit;
  try {
    localStorage.setItem("cheki-image-fit", fit);
  } catch {}
}
