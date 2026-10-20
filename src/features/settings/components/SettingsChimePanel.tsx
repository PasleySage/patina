import { useLocaleText } from "../../../shared/i18n/index.ts";
import { BellRing } from "lucide-react";
import QuietActionRow from "../../../shared/components/QuietActionRow";
import QuietButton from "../../../shared/components/QuietButton";
import QuietSelect from "../../../shared/components/QuietSelect";
import QuietStepperSlider from "../../../shared/components/QuietStepperSlider";
import QuietSwitch from "../../../shared/components/QuietSwitch";
import SettingsPanelHeader from "./SettingsPanelHeader";
import { SettingsPreferenceGroup, SettingsPreferenceRow } from "./SettingsPreferenceLayout";
import { CHIME_TONE_OPTIONS, CHIME_VOLUME_MAX, CHIME_VOLUME_MIN, type ChimeTone } from "../services/chimeSettingsService.ts";

type SettingsChimePanelProps = {
  enabled: boolean;
  volume: number;
  tone: ChimeTone;
  busy: boolean;
  loadFailed: boolean;
  onEnabledChange: (nextChecked: boolean) => void;
  onVolumeChange: (nextVolume: number) => void;
  onToneChange: (nextTone: ChimeTone) => void;
  onPreview: () => void;
};

export default function SettingsChimePanel({
  enabled,
  volume,
  tone,
  busy,
  loadFailed,
  onEnabledChange,
  onVolumeChange,
  onToneChange,
  onPreview,
}: SettingsChimePanelProps) {
  const UI_TEXT = useLocaleText();
  const toneLabels = UI_TEXT.settings.chimeToneLabels;
  return (
    <section className="qp-panel p-5 md:p-6">
      <SettingsPanelHeader
        icon={<BellRing size={16} className="text-[var(--qp-accent-default)]" />}
        title={UI_TEXT.settings.chimeTitle}
      />

      {loadFailed ? (
        <p role="alert" className="mt-3 text-xs text-[var(--qp-danger-default)]">
          {UI_TEXT.settings.chimeUnavailable}
        </p>
      ) : null}

      <div className="mt-5 space-y-5">
        <SettingsPreferenceGroup title={UI_TEXT.settings.chimeGroupTitle}>
          <QuietActionRow className="settings-preference-list">
            <SettingsPreferenceRow title={UI_TEXT.settings.chimeEnabledLabel}>
              <QuietSwitch
                checked={enabled}
                onChange={onEnabledChange}
                ariaLabel={UI_TEXT.accessibility.settings.toggleChime}
              />
            </SettingsPreferenceRow>
            <SettingsPreferenceRow
              title={UI_TEXT.settings.chimeVolumeLabel}
              hint={UI_TEXT.settings.chimeVolumeHint}
            >
              <QuietStepperSlider
                ariaLabel={UI_TEXT.settings.chimeVolumeLabel}
                value={volume}
                min={CHIME_VOLUME_MIN}
                max={CHIME_VOLUME_MAX}
                step={5}
                displayValue={`${volume}%`}
                decreaseAriaLabel={UI_TEXT.settings.chimeVolumeDecrease}
                increaseAriaLabel={UI_TEXT.settings.chimeVolumeIncrease}
                onChange={onVolumeChange}
              />
            </SettingsPreferenceRow>
            <SettingsPreferenceRow
              title={UI_TEXT.settings.chimeToneLabel}
              hint={UI_TEXT.settings.chimeToneHint}
            >
              <div className="flex items-center gap-2">
                {/* Volume and tone stay adjustable while the chime is off: a user
                    normally picks a tone and a level first, then turns it on. Only the
                    preview is gated, because playing it is the thing being disabled. */}
                <QuietSelect<ChimeTone>
                  value={tone}
                  ariaLabel={UI_TEXT.settings.chimeToneLabel}
                  density="compact"
                  onChange={onToneChange}
                  options={CHIME_TONE_OPTIONS.map((option) => ({
                    value: option,
                    label: toneLabels[option],
                  }))}
                />
                <QuietButton
                  onClick={onPreview}
                  disabled={volume === 0}
                  busy={busy}
                  size="compact"
                >
                  {UI_TEXT.settings.chimePreviewAction}
                </QuietButton>
              </div>
            </SettingsPreferenceRow>
          </QuietActionRow>
        </SettingsPreferenceGroup>
      </div>
    </section>
  );
}
