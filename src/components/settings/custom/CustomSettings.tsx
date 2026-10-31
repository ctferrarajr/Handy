import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";

export const CustomSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title={t("settings.custom.title")}>
        <ToggleSwitch
          checked={getSetting("spoken_punctuation_enabled") ?? true}
          onChange={(enabled) =>
            updateSetting("spoken_punctuation_enabled", enabled)
          }
          isUpdating={isUpdating("spoken_punctuation_enabled")}
          label={t("settings.custom.spokenPunctuation.title")}
          description={t("settings.custom.spokenPunctuation.description")}
          descriptionMode="inline"
          grouped={true}
        />
        <ToggleSwitch
          checked={getSetting("smart_capitalization_enabled") ?? true}
          onChange={(enabled) =>
            updateSetting("smart_capitalization_enabled", enabled)
          }
          isUpdating={isUpdating("smart_capitalization_enabled")}
          label={t("settings.custom.smartCapitalization.title")}
          description={t("settings.custom.smartCapitalization.description")}
          descriptionMode="inline"
          grouped={true}
        />
      </SettingsGroup>
    </div>
  );
};
