import { forwardRef, useState, type ReactNode } from "react";
import type { AppActions } from "../../lib/appData";
import { describeError } from "../../lib/backend";
import type { LimitsConnection, LimitsView } from "../../lib/contracts";
import { formatClock } from "../../lib/format";
import { TextLink } from "../../components/TextLink";

interface LimitDataProps {
  limits: LimitsView;
  connection: LimitsConnection;
  connectLimits: AppActions["connectLimits"];
  disconnectLimits: AppActions["disconnectLimits"];
}

const BLOCKED_TEXT: Record<NonNullable<LimitsConnection["problem"]>, string> = {
  noClaudeFolder: "Can’t connect: the Claude folder isn’t there. Pick it below.",
  settingsUnreadable: "Can’t connect: Claude Code’s settings.json can’t be read.",
  settingsInvalid: "Can’t connect: settings.json isn’t valid JSON. Fix it, then try again.",
};

function connectedLine(limits: LimitsView): ReactNode {
  if (limits.status === "unavailable" || limits.fetchedAt === null) return "Connected. Limits after your next reply.";
  return (
    <>
      Connected, last reading <span className="live-number">{formatClock(limits.fetchedAt)}</span>
    </>
  );
}

/**
 * Settings, "Limits" row: one-click Connect or Disconnect of Claude Code's
 * status line (the app writes `settings.json` itself, with a backup).
 */
export const LimitData = forwardRef<HTMLElement, LimitDataProps>(function LimitData(
  { limits, connection, connectLimits, disconnectLimits },
  ref,
) {
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  const run = async (action: () => Promise<LimitsConnection>) => {
    setBusy(true);
    setFailed(false);
    try {
      await action();
    } catch (error) {
      console.warn(`limits connection change failed: ${describeError(error)}`);
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  const link = (text: string, action: () => Promise<LimitsConnection>) => (
    <TextLink onClick={() => void run(action)} disabled={busy}>
      {text}
    </TextLink>
  );

  let sub: ReactNode;
  let links: ReactNode;
  if (connection.state === "connected") {
    sub = connectedLine(limits);
    links = link("Disconnect", disconnectLimits);
  } else if (connection.state === "outdated") {
    sub = "Connected, but set up for another shell. Reconnect to fix it.";
    links = (
      <>
        {link("Reconnect", connectLimits)}
        {link("Disconnect", disconnectLimits)}
      </>
    );
  } else if (connection.state === "blocked") {
    sub = connection.problem === null ? "Can’t connect right now." : BLOCKED_TEXT[connection.problem];
    links = link("Try again", connectLimits);
  } else {
    sub = "Not connected.";
    links = link("Connect limits", connectLimits);
  }

  return (
    <section ref={ref} className="row" aria-labelledby="settings-limit-data">
      <div className="row__main">
        <div className="row__text">
          <span id="settings-limit-data" className="row__label">
            Limits
          </span>
          <span className="caption">{sub}</span>
        </div>
        <span className="row__links">{links}</span>
      </div>
      <div className="row__captions">
        {connection.state === "notConnected" ? (
          <p className="caption">
            Limits come from Claude Code’s status line. Connect adds it to{" "}
            <span className="path">{connection.settingsPath ?? "settings.json"}</span>, with a backup.
            {connection.previousStatusLine ? " Your current status line keeps working, output unchanged." : ""}
          </p>
        ) : null}
        {connection.state === "connected" && connection.previousStatusLine ? (
          <p className="caption">Your own status line runs on, output unchanged. Disconnect restores it.</p>
        ) : null}
        {connection.state === "outdated" && connection.previousStatusLine ? (
          <p className="caption">Your own status line stays wrapped; Reconnect keeps it.</p>
        ) : null}
        <p className="caption">
          Needs a Pro or Max plan. Numbers show up after the first reply; the desktop app may not run the status
          line. Limit % includes claude.ai chats. Token stats are Claude Code only.
        </p>
        {failed ? (
          <p className="note note--error" role="alert">
            Couldn’t change that.
          </p>
        ) : null}
      </div>
    </section>
  );
});
