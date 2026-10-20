import { useLocaleText } from "../../../shared/i18n/index.ts";
import appIconUrl from "../../../../src-tauri/icons/icon.png";

type AboutPanelProps = {
  appVersion: string;
};

export default function AboutPanel({ appVersion }: AboutPanelProps) {
  const UI_TEXT = useLocaleText();
  const versionLabel = appVersion === "-" || appVersion === "unknown"
    ? UI_TEXT.about.versionUnknown
    : `v${appVersion}`;
  return (
    <div className="about-center-workbench">
      <section className="qp-panel about-center-panel">
        <div className="about-center-profile">
          <div className="about-center-icon-shell" aria-hidden>
            <img src={appIconUrl} alt="" draggable={false} />
          </div>
          <div className="about-center-title-row">
            <h2>Patina</h2>
            <span className="about-center-version qp-text-caption qp-weight-emphasis">{versionLabel}</span>
          </div>
          <p>{UI_TEXT.about.description}</p>
        </div>
      </section>
    </div>
  );
}
