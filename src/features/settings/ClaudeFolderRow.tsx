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

  const source = folder === null ? "" : SOURCE_LABEL[folder.source];
  const missing = folder !== null && !folder.projectsFound;

  return (
    <section className="row" aria-labelledby="settings-claude-folder">
      <div className="row__main">
        <div className="row__text">
          <span id="settings-claude-folder" className="row__label">
            Claude folder
          </span>
          <span className="meta row__path" title={folder?.path ?? undefined}>
            {folder === null ? "…" : folder.path === null ? "Unknown" : truncateMiddle(folder.path)}
          </span>
        </div>
        <span className="row__links">
          <TextLink onClick={() => void choose()} disabled={busy}>
            Change
          </TextLink>
          {folder?.source === "setting" ? (
            <TextLink onClick={() => void run(() => onChange(null))} disabled={busy}>
              Reset
            </TextLink>
          ) : null}
        </span>
      </div>
      {source !== "" || missing || failed ? (
        <div className="row__captions">
          {source !== "" ? <p className="caption">{source}</p> : null}
          {missing ? <p className="caption">No logs here yet.</p> : null}
          {failed ? (
            <p className="note note--error" role="alert">
              Couldn’t save that. Try again.
            </p>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
