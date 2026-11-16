import { useState } from "react";
import { TextLink } from "../../components/TextLink";
import type { ClaudeFolder, ClaudeFolderSource } from "../../lib/contracts";

interface ClaudeFolderRowProps {
  /** `null` until the first read answered (or when the core can't answer). */
  folder: ClaudeFolder | null;
  /** Sets the folder (string) or resets it (`null`). Resolves `false` when rejected. */
  onChange: (claudeFolder: string | null) => Promise<boolean>;
  /** Opens the system folder picker; `null` when cancelled. */
  pickFolder: () => Promise<string | null>;
}

const SOURCE_LABEL: Record<ClaudeFolderSource, string> = {
  setting: "Set here",
  env: "From CLAUDE_CONFIG_DIR",
  default: "Default",
};

const MAX_CHARS = 38;

/** Keeps both ends of a long path, the part that tells folders apart. */
export function truncateMiddle(path: string, max = MAX_CHARS): string {
  if (path.length <= max) return path;
  const keep = max - 1;
  const head = Math.ceil(keep / 2);
  const tail = Math.floor(keep / 2);
  return `${path.slice(0, head)}…${path.slice(path.length - tail)}`;
}

/** Settings row: which Claude folder Tokenmonk reads, and where that came from. */
export function ClaudeFolderRow({ folder, onChange, pickFolder }: ClaudeFolderRowProps) {
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  const run = async (action: () => Promise<boolean>) => {
    setBusy(true);
    setFailed(false);
    try {
      if (!(await action())) setFailed(true);
    } catch {
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  const choose = () =>
    run(async () => {
      const picked = await pickFolder();
      return picked === null ? true : onChange(picked);
    });

  return (
    <section className="folder-row" aria-labelledby="settings-claude-folder">
      <span id="settings-claude-folder" className="label">
        Claude folder
      </span>
      <p className="folder-row__path live-number" title={folder?.path ?? undefined}>
        {folder === null ? "…" : folder.path === null ? "Unknown" : truncateMiddle(folder.path)}
      </p>
      <p className="caption folder-row__meta">
        <span>{folder === null ? "" : SOURCE_LABEL[folder.source]}</span>
        <span className="folder-row__actions">
          <TextLink small onClick={() => void choose()} disabled={busy}>
            Choose…
          </TextLink>
          {folder?.source === "setting" ? (
            <TextLink small onClick={() => void run(() => onChange(null))} disabled={busy}>
              Reset
            </TextLink>
          ) : null}
        </span>
      </p>
      {folder !== null && !folder.projectsFound ? <p className="caption">No logs here yet.</p> : null}
      {failed ? (
        <p className="note note--error" role="alert">
          Couldn’t save that. Try again.
        </p>
      ) : null}
    </section>
  );
}
