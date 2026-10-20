import { useLocaleText } from "../../../shared/i18n/index.ts";
import { useEffect, useState } from "react";
import { Info } from "lucide-react";

import QuietPageHeader from "../../../shared/components/QuietPageHeader";
import AboutPanel from "./AboutPanel";
import {
  getSettingsPageBootstrapCache,
  prewarmSettingsBootstrapCache,
} from "../../settings/services/settingsBootstrapService.ts";

export default function About() {
  const UI_TEXT = useLocaleText();
  const cachedBootstrap = getSettingsPageBootstrapCache();
  const [appVersion, setAppVersion] = useState(cachedBootstrap?.appVersion ?? "-");

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const bootstrap = await prewarmSettingsBootstrapCache();
        if (!cancelled) {
          setAppVersion(bootstrap.appVersion);
        }
      } catch (error) {
        console.error("load about bootstrap failed", error);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="flex h-full min-w-0 flex-col gap-4 md:gap-5">
      <QuietPageHeader
        icon={<Info size={18} />}
        title={UI_TEXT.about.title}
      />

      <div className="flex-1 overflow-y-auto qp-scroll-region pr-2">
        <AboutPanel appVersion={appVersion} />
      </div>
    </div>
  );
}
