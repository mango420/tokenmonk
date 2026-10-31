import { useEffect, useRef, useState } from "react";
import { Icon } from "../../components/Icon";

type Feedback = "copied" | "failed" | null;

const COPIED_MS = 2_000;

interface SnippetCardProps {
  /** Selectable text; what the copy button puts on the clipboard. */
  text: string;
  /** Accessible name of the copy button, for example "Copy snippet". */
  copyLabel: string;
}

const COPIED_CAPTION = "Copied. Paste it, then send Claude Code one message.";

/**
 * Snippet card (`surface-200`, mono) with a copy button in the top right. A
 * failed copy selects the text and says so calmly.
 */
export function SnippetCard({ text, copyLabel }: SnippetCardProps) {
  const [feedback, setFeedback] = useState<Feedback>(null);
  const textRef = useRef<HTMLPreElement>(null);
  const timer = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (timer.current !== null) window.clearTimeout(timer.current);
    },
    [],
  );

  const selectText = () => {
    const node = textRef.current;
    const selection = window.getSelection();
    if (node === null || selection === null) return;
    const range = document.createRange();
    range.selectNodeContents(node);
    selection.removeAllRanges();
    selection.addRange(range);
  };

  const copy = async () => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(text);
      setFeedback("copied");
      timer.current = window.setTimeout(() => setFeedback(null), COPIED_MS);
    } catch {
      setFeedback("failed");
      selectText();
    }
  };

  return (
    <>
      <div className="snippet">
        <pre ref={textRef} className="snippet__text">
          {text}
        </pre>
        <button type="button" className="icon-button snippet__copy" aria-label={copyLabel} onClick={() => void copy()}>
          <Icon name="copy" />
        </button>
      </div>
      {feedback === null ? null : (
        <span className="caption" role="status">
          {feedback === "copied" ? COPIED_CAPTION : "Couldn’t copy. Select it and copy by hand."}
        </span>
      )}
    </>
  );
}
