import { LogoMark } from "../../components/LogoMark";
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
    <div className="screen">
      <TopRow>
        <TextLink onClick={onBack}>Back</TextLink>
      </TopRow>
      <div className="scroll">
        <div className="about__mark">
          <LogoMark />
        </div>
        <h1 className="title about__title">Tokenmonk</h1>
        <p className="about__version live-number">{version === "" ? "n/a" : version}</p>
        <p className="caption about__caption">Installed version. Updates live in Settings.</p>
        <div className="about__row">
          <span>Licence</span>
          <span className="muted">MIT</span>
        </div>
        <div className="about__row">
          <span>Source</span>
          <TextLink onClick={() => void openExternal(REPO_URL)}>GitHub</TextLink>
        </div>
        <div className="about__row">
          <span>Ideas and bugs</span>
          <TextLink onClick={() => void openExternal(ISSUES_URL)}>GitHub issues</TextLink>
        </div>
        <div className="about__end" />
      </div>
    </div>
  );
}
