// Issue title normalisation shared by .github/workflows/issue-key.yml.
//
// Every issue title is "TKM-<n>: <rest>". A leading "TKM-<m>" with or without a
// colon (m may be stale) is replaced, never doubled. Nothing else is touched.

const LEADING_KEY = /^TKM-(\d+)(?:\s*:\s*|\s+|$)/i;

// Title without its leading key, whatever the separator form.
export function stripKey(title) {
  return title.trim().replace(LEADING_KEY, '').trim();
}

export function normalizeTitle(title, n) {
  const key = `TKM-${n}`;
  const rest = stripKey(title);
  return rest ? `${key}: ${rest}` : key;
}
