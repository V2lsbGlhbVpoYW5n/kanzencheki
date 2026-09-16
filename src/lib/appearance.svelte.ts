export type Appearance = "system" | "light" | "dark";
export const appearance = $state<{ mode: Appearance }>({ mode: "system" });
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
    const saved = localStorage.getItem("cheki-appearance");
    if (saved === "dark" || saved === "light" || saved === "system")
      appearance.mode = saved;
  } catch {}
  const media = matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", applyAppearance);
  applyAppearance();
  return () => media.removeEventListener("change", applyAppearance);
}
