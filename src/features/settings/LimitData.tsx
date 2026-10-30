import { forwardRef, useEffect, useState } from "react";
import type { AppActions } from "../../lib/appData";
import { describeError } from "../../lib/backend";
import type { LimitSetup, LimitsView, Platform } from "../../lib/contracts";
import { formatClock } from "../../lib/format";
import { SnippetCard } from "./SnippetCard";

interface LimitDataProps {
  limits: LimitsView;
  platform: Platform;
  loadLimitSetup: AppActions["loadLimitSetup"];
}

type Setup = { state: "loading" } | { state: "failed" } | { state: "ready"; setup: LimitSetup };

function StatusLine({ limits }: { limits: LimitsView }) {
  if (limits.status !== "unavailable") {
    return (
      <p className="label-strong">
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
      ? "Not set up yet."
      : "Can’t read the limit file. Next reply fixes it.";
  return <p className="label-strong">{text}</p>;
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

  useEffect(() => {
    let cancelled = false;
    loadLimitSetup().then(
      (value) => {
        if (!cancelled) setSetup({ state: "ready", setup: value });
      },
      (error: unknown) => {
        console.warn(`get_limit_setup failed: ${describeError(error)}`);
        if (!cancelled) setSetup({ state: "failed" });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [loadLimitSetup]);

  const command = setup.state === "ready" ? setup.setup.statuslineCommand : null;
  // A quoted path needs the call operator in PowerShell, which Git Bash can't parse.
  const powerShellAlternative = platform === "windows" && command !== null && command.startsWith('"');

  return (
    <section ref={ref} className="limit-data" aria-labelledby="settings-limit-data">
      <span id="settings-limit-data" className="caption">
        Limit data
      </span>
      <StatusLine limits={limits} />
      <p className="caption">
        Limits come from Claude Code’s status line. Local only, no login. Add this to{" "}
        <span className="limit-data__path">~/.claude/settings.json</span>:
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
          {powerShellAlternative ? (
            <>
              <p className="caption">No Git Bash? Claude Code then uses PowerShell. Use this one instead:</p>
              <SnippetCard text={snippetFor(`& ${command}`)} copyLabel="Copy PowerShell snippet" />
            </>
          ) : null}
          <p className="caption">
            Already have a status line? Keep it. Add this line to your script after it reads the input:
          </p>
          <SnippetCard
            text={`printf '%s' "$input" | ${command} >/dev/null`}
            copyLabel="Copy chain line"
          />
        </>
      )}

      <p className="caption">Needs a Pro or Max plan. Numbers show up after the first reply.</p>
    </section>
  );
});
