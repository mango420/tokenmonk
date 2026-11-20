import { FiveBeadGauge } from "../../components/FiveBeadGauge";
import { TextLink } from "../../components/TextLink";
import { TopRow } from "../../components/TopRow";
import { openExternal } from "../../lib/backend";
import { ISSUES_URL, REPO_URL } from "../../lib/contracts";
import { useEscape } from "../../lib/useEscape";

interface AboutViewProps {
  /** `AppInfo.version` as is; empty while it is still loading. */
  version: string;
  onBack: () => void;
}

/** About: version, licence and the GitHub links (docs/DESIGN.md, About). */
export function AboutView({ version, onBack }: AboutViewProps) {
  useEscape(onBack);

  return (
    <div className="screen screen--bands">
      <div className="band band--centered">
        <TopRow corner="wordmark">
          <TextLink onClick={onBack}>Back</TextLink>
        </TopRow>
        <FiveBeadGauge />
        <p className="about__version live-number">{version === "" ? "n/a" : version}</p>
        <p className="small about__caption">Installed version. Updates live in Settings.</p>
      </div>
      <div className="body body--about">
        <div className="hairlist">
          <div className="about__row about__row--first">
            <span>Licence</span>
            <span className="about__value">MIT</span>
          </div>
          <div className="about__row">
            <span>Source</span>
            <TextLink onClick={() => void openExternal(REPO_URL)}>GitHub</TextLink>
          </div>
          <div className="about__row">
            <span>Ideas and bugs</span>
            <TextLink onClick={() => void openExternal(ISSUES_URL)}>GitHub issues</TextLink>
          </div>
        </div>
        <div className="spacer" />
        <p className="note">Made by Tokenmonk contributors. No account, no telemetry.</p>
      </div>
    </div>
  );
}
