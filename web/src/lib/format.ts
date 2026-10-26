export function bytes(n: number) {
  if (!n) return "0 B";
  const i = Math.min(4, Math.floor(Math.log(n) / Math.log(1024)));
  return `${(n / 1024 ** i).toLocaleString(undefined, { maximumFractionDigits: 1 })} ${["B", "KiB", "MiB", "GiB", "TiB"][i]}`;
}
export const fileCount = (n: number) => `${n} file${n === 1 ? "" : "s"}`;
export const date = (s: number | null) =>
  s
    ? new Date(s * 1000).toLocaleString(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      })
    : "No deadline";
export function localDate(t: number) {
  const d = new Date(t * 1000);
  return new Date(d.getTime() - d.getTimezoneOffset() * 60000)
    .toISOString()
    .slice(0, 16);
}
export function optionalNumber(value: FormDataEntryValue | null) {
  return value ? Number(value) : null;
}
