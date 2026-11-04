import { forwardRef, useCallback, useEffect, useRef, useState } from "react";
import type { AppActions } from "../../lib/appData";
import { describeError } from "../../lib/backend";
import type { LimitSetup, LimitsView, Platform, StatuslineFileInfo } from "../../lib/contracts";
import { formatClock, formatDayTime } from "../../lib/format";
import { TextLink } from "../../components/TextLink";
import { SnippetCard } from "./SnippetCard";

interface LimitDataProps {
  limits: LimitsView;
  platform: Platform;
  loadLimitSetup: AppActions["loadLimitSetup"];
}

type Setup = { state: "loading" } | { state: "failed" } | { state: "ready"; setup: LimitSetup };

function StatusLine({ limits, received }: { limits: LimitsView; received: boolean }) {
  if (limits.status !== "unavailable") {
    return (
      <p className="limit-data__status">
        {limits.fetchedAt === null ? (
          "Connected."
        ) : (
          <>
            Connected. Last reading{" "}
            <span className="live-number">{formatClock(limits.fetchedAt)}</span>.
          </>
        )}
      </p>
    );
  }
  const text =
    limits.reason === "noSource" || limits.reason === null
      ? received
        ? "Set up. No current limits."
        : "Not set up yet."
      : "Can’t read the limit file. Next reply fixes it.";
  return <p className="limit-data__status">{text}</p>;
}

function CheckLine({ file, onCheck }: { file: StatuslineFileInfo; onCheck: () => void }) {
  let body;
  if (!file.exists) {
    body = "Nothing from the status line yet. Paste the block below, then send Claude Code one message.";
  } else if (file.receivedAt !== null) {
    body = (
      <>
        <span className="live-number">{formatDayTime(file.receivedAt)}</span> · last status line. No current
        window in it: the next reply brings one on Pro or Max.
      </>
    );
  } else if (file.modifiedAt !== null) {
    body = (
      <>
        <span className="live-number">{formatDayTime(file.modifiedAt)}</span> · something arrived, but it can’t
        be read. Send another message; if it sticks, update Tokenmonk.
      </>
    );
  } else {
    body = "Something arrived, but it can’t be read. Send another message; if it sticks, update Tokenmonk.";
  }
  return (
    <p className="caption limit-data__check">
      {body}{" "}
      <TextLink small onClick={onCheck}>
        Check again
      </TextLink>
    </p>
  );
}

function snippetFor(command: string): string {
  return JSON.stringify({ statusLine: { type: "command", command } }, null, 2);
}

/**
 * Settings, "Limit data": the status-line snippet to paste into Claude Code's
 * settings. Tokenmonk never edits those settings itself.
 */
export const LimitData = forwardRef<HTMLElement, LimitDataProps>(function LimitData(
  { limits, platform, loadLimitSetup },
  ref,
) {
  const [setup, setSetup] = useState<Setup>({ state: "loading" });

  const alive = useRef(true);

  const load = useCallback(
    (keepOnFailure: boolean) => {
      loadLimitSetup().then(
        (value) => {
          if (alive.current) setSetup({ state: "ready", setup: value });
        },
        (error: unknown) => {
          console.warn(`get_limit_setup failed: ${describeError(error)}`);
          if (alive.current && !keepOnFailure) setSetup({ state: "failed" });
        },
      );
    },
    [loadLimitSetup],
  );

  useEffect(() => {
    alive.current = true;
    load(false);
    return () => {
      alive.current = false;
    };
  }, [load]);

  const command = setup.state === "ready" ? setup.setup.statuslineCommand : null;
  // A quoted path needs the call operator in PowerShell, which Git Bash can't parse.
  const file = setup.state === "ready" ? setup.setup.statuslineFile : null;
  const windows = platform === "windows";
  const powerShellAlternative = windows && command !== null && command.startsWith('"');
  const settingsPath = windows ? "%USERPROFILE%\\.claude\\settings.json" : "~/.claude/settings.json";

  return (
    <section ref={ref} className="limit-data" aria-labelledby="settings-limit-data">
      <span id="settings-limit-data" className="label">
        Limit data
      </span>
      <StatusLine limits={limits} received={file?.receivedAt != null} />
      {file !== null && limits.status === "unavailable" ? (
        <CheckLine file={file} onCheck={() => load(true)} />
      ) : null}
      <p className="caption">
        Limits come from Claude Code’s status line. Local only, no login. Open{" "}
        <span className="limit-data__path">{settingsPath}</span>, create it if missing, and add this as your status line:
      </p>

      {setup.state === "loading" ? null : setup.state === "failed" ? (
        <p className="caption" role="status">
          Couldn’t look up the setup line. Close Settings and open it again.
        </p>
      ) : command === null ? (
        <p className="caption" role="status">
          Couldn’t find the Tokenmonk executable. Reinstall, then look again.
        </p>
      ) : (
        <>
          <SnippetCard text={snippetFor(command)} copyLabel="Copy snippet" />
          <p className="caption">
            File already has settings? Put <span className="limit-data__path">"statusLine"</span> at the top
            level next to them and end the key before it with a comma. Already has a{" "}
            <span className="limit-data__path">"statusLine"</span>? This replaces it.
            {windows ? "" : " To keep your own script, skip this block and use the line below."}
          </p>
          {powerShellAlternative ? (
            <>
              <p className="caption">No Git Bash? Claude Code then uses PowerShell. Use this one instead:</p>
              <SnippetCard text={snippetFor(`& ${command}`)} copyLabel="Copy PowerShell snippet" />
            </>
          ) : null}
          {windows ? (
            <p className="caption">
              Already have a status line? Tokenmonk can’t share it on Windows yet. Swap yours for the block
              above, or keep it and go without limits.
            </p>
          ) : (
            <>
              <p className="caption">
                Already have a status line script? Leave{" "}
                <span className="limit-data__path">settings.json</span> as it is. Add this line inside the
                script, after it reads the input:
              </p>
              <SnippetCard text={`printf '%s' "$input" | ${command} >/dev/null`} copyLabel="Copy chain line" />
            </>
          )}
        </>
      )}

      <p className="caption">Needs a Pro or Max plan. Numbers show up after the first reply.</p>
    </section>
  );
});
