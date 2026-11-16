import { forwardRef, useCallback, useEffect, useState } from "react";
import type { AppActions } from "../../lib/appData";
import { describeError, useBackendEvent } from "../../lib/backend";
import { Events, type LimitsConnection, type LimitsView } from "../../lib/contracts";
import { formatClock } from "../../lib/format";
import { TextLink } from "../../components/TextLink";

interface LimitDataProps {
  limits: LimitsView;
  loadLimitsConnection: AppActions["loadLimitsConnection"];
  connectLimits: AppActions["connectLimits"];
  disconnectLimits: AppActions["disconnectLimits"];
}

type Link = { state: "loading" } | { state: "failed" } | { state: "ready"; connection: LimitsConnection };

const BLOCKED_TEXT: Record<NonNullable<LimitsConnection["problem"]>, string> = {
  noClaudeFolder: "Can’t connect: the Claude folder isn’t there. Pick it above.",
  settingsUnreadable: "Can’t connect: Claude Code’s settings.json can’t be read.",
  settingsInvalid: "Can’t connect: settings.json isn’t valid JSON. Fix it, then try again.",
};

function readingLine(limits: LimitsView) {
  if (limits.status === "unavailable" || limits.fetchedAt === null) return "Connected. Limits after your next reply.";
  return (
    <>
      Connected. Last reading <span className="live-number">{formatClock(limits.fetchedAt)}</span>.
    </>
  );
}

/**
 * Settings, "Limits": one-click Connect or Disconnect of Claude Code's status
 * line (the app writes `settings.json` itself, with a backup).
 */
export const LimitData = forwardRef<HTMLElement, LimitDataProps>(function LimitData(
  { limits, loadLimitsConnection, connectLimits, disconnectLimits },
  ref,
) {
  const [link, setLink] = useState<Link>({ state: "loading" });
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);

  const load = useCallback(() => {
    loadLimitsConnection().then(
      (connection) => setLink({ state: "ready", connection }),
      (error: unknown) => {
        console.warn(`get_limits_connection failed: ${describeError(error)}`);
        setLink({ state: "failed" });
      },
    );
  }, [loadLimitsConnection]);
  useEffect(load, [load]);
  useBackendEvent(Events.limitsConnectionChanged, (connection) => setLink({ state: "ready", connection }));

  const run = async (action: () => Promise<LimitsConnection>) => {
    setBusy(true);
    setFailed(false);
    try {
      setLink({ state: "ready", connection: await action() });
    } catch (error) {
      console.warn(`limits connection change failed: ${describeError(error)}`);
      setFailed(true);
      load();
    } finally {
      setBusy(false);
    }
  };

  const connection = link.state === "ready" ? link.connection : null;
  let status;
  let action = null;
  if (link.state === "loading") {
    status = "…";
  } else if (connection === null) {
    status = "Couldn’t read the Limits setup. Close Settings and open it again.";
  } else if (connection.state === "connected") {
    status = readingLine(limits);
    action = (
      <TextLink small onClick={() => void run(disconnectLimits)} disabled={busy}>
        Disconnect
      </TextLink>
    );
  } else if (connection.state === "blocked") {
    status = connection.problem === null ? "Can’t connect right now." : BLOCKED_TEXT[connection.problem];
    action = (
      <TextLink small onClick={load} disabled={busy}>
        Check again
      </TextLink>
    );
  } else {
    status = "Not connected.";
    action = (
      <TextLink small onClick={() => void run(connectLimits)} disabled={busy}>
        Connect limits
      </TextLink>
    );
  }

  return (
    <section ref={ref} className="limit-data" aria-labelledby="settings-limit-data">
      <span id="settings-limit-data" className="label">
        Limits
      </span>
      <p className="limit-data__status">{status}</p>
      {action === null ? null : <div className="limit-data__actions">{action}</div>}
      {connection?.state === "notConnected" ? (
        <p className="caption">
          Limits come from Claude Code’s status line. Connect adds it to{" "}
          <span className="limit-data__path">{connection.settingsPath ?? "settings.json"}</span>, with a backup.
          {connection.previousStatusLine ? " Your current status line keeps working, output unchanged." : ""}
        </p>
      ) : null}
      {connection?.state === "connected" && connection.previousStatusLine ? (
        <p className="caption">Your own status line runs on, output unchanged. Disconnect restores it.</p>
      ) : null}
      <p className="caption">
        Needs a Pro or Max plan. Numbers show up after the first reply; the desktop app may not run the status line.
      </p>
      {failed ? (
        <p className="note note--error" role="alert">
          Couldn’t change that. Try again.
        </p>
      ) : null}
    </section>
  );
});
