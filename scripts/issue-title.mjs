// Issue title normalisation shared by .github/workflows/issue-key.yml.
//
// Every issue title is "TKM-<n>: <rest>". A leading "TKM-<m>" with or without a
// colon (m may be stale) is replaced, never doubled. Nothing else is touched.

const LEADING_KEY = /^TKM-(\d+)(?:\s*:\s*|\s+|$)/i;

export function normalizeTitle(title, n) {
  const key = `TKM-${n}`;
  const text = title.trim();
  const m = LEADING_KEY.exec(text);
  const rest = (m ? text.slice(m[0].length) : text).trim();
  return rest ? `${key}: ${rest}` : key;
}
