/**
 * Typed bridge to the Rust core. Command and event names come from
 * `contracts.ts`; nothing else in the UI imports `@tauri-apps/api` for data.
 */
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import type {
  CommandArgs,
  CommandError,
  CommandName,
  CommandResults,
  EventName,
  EventPayloads,
} from "./contracts";

/** True inside the Tauri webview; false in a plain browser (dev preview). */
export function insideTauri(): boolean {
  return isTauri();
}

/** Calls a backend command with the arguments its contract names. */
export function call<C extends CommandName>(
  command: C,
  ...args: CommandArgs[C] extends undefined ? [] : [CommandArgs[C]]
): Promise<CommandResults[C]> {
  const payload = args[0] as Record<string, unknown> | undefined;
  return invoke<CommandResults[C]>(command, payload);
}

/** Readable text for a rejected command; the UI never shows it, it only logs it. */
export function describeError(error: unknown): string {
  if (typeof error === "string") return error;
  if (typeof error === "object" && error !== null && "message" in error) {
    const { message } = error as Partial<CommandError>;
    if (typeof message === "string") return message;
  }
  return "unknown error";
}

/**
 * Subscribes to a backend event while the component is mounted. The handler
 * may change on every render; the subscription does not.
 */
export function useBackendEvent<E extends EventName>(
  event: E,
  handler: (payload: EventPayloads[E]) => void,
  enabled = true,
): void {
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    if (!enabled || !insideTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | null = null;
    listen<EventPayloads[E]>(event, (e) => latest.current(e.payload))
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      })
      .catch((error: unknown) => {
        console.warn(`could not listen to ${event}: ${describeError(error)}`);
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [event, enabled]);
}
