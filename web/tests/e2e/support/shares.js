export async function seed(page, options = {}) {
  return page.evaluate(async (options) => {
    const me = await (await fetch("/api/me")).json();
    const headers = {
      "content-type": "application/json",
      "x-binbin-csrf": me.csrf,
    };
    const response = await fetch("/api/shares", {
      method: "POST",
      headers,
      body: JSON.stringify({
        title: options.title || "Design review files",
        files: [{ name: options.filename || "handover.txt", size: 5 }],
        days: 7,
        limit: options.limit ?? 10,
        password: options.password || null,
      }),
    });
    if (!response.ok) throw new Error(await response.text());
    const draft = await response.json();
    await fetch(`/api/files/${draft.files[0].id}?offset=0`, {
      method: "PATCH",
      headers: { "x-binbin-csrf": me.csrf },
      body: options.partial ? "he" : "hello",
    });
    if (options.partial) return draft;
    const published = await fetch(`/api/shares/${draft.id}/publish`, {
      method: "POST",
      headers,
    });
    if (!published.ok) throw new Error(await published.text());
    return published.json();
  }, options);
}
