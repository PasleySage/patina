import assert from "node:assert/strict";
import {
  buildUpdateConfirmDialogModel as buildUpdateConfirmDialogModelRaw,
  shouldOpenUpdateDialogForSnapshot,
  shouldShowCompactUpdateEntry,
} from "../src/features/update/services/updateViewModel.ts";
import {
  clearPendingUpdateRelaunchViewRestore,
  consumePendingUpdateRelaunchView,
  markPendingUpdateRelaunchViewRestore,
  readLastActiveView,
  rememberLastActiveView,
} from "../src/app/services/updateRelaunchViewStorage.ts";
import { getLocaleText, loadLocaleText } from "../src/shared/i18n/runtime.ts";
import type { UpdateSnapshot } from "../src/shared/types/update.ts";
import { MemoryStorage, withWindowStorage } from "./helpers/browserTestGlobals.ts";

const ZH_TEXT = getLocaleText("zh-CN");
const EN_TEXT = await loadLocaleText("en-US");
const buildUpdateConfirmDialogModel = (snapshot: UpdateSnapshot) => (
  buildUpdateConfirmDialogModelRaw(snapshot, ZH_TEXT, "zh-CN")
);

function makeSnapshot(overrides: Partial<UpdateSnapshot> = {}): UpdateSnapshot {
  return {
    currentVersion: "0.1.0",
    status: "idle",
    latestVersion: null,
    releaseNotes: null,
    releaseDate: null,
    errorMessage: null,
    errorStage: null,
    downloadedBytes: null,
    totalBytes: null,
    releasePageUrl: "https://github.com/Ceceliaee/patina/releases",
    assetDownloadUrl: null,
    ...overrides,
  };
}

let passed = 0;

function runTest(name: string, fn: () => void) {
  fn();
  passed += 1;
  console.log(`PASS ${name}`);
}

// The compact shell entry is the only update affordance left outside the dialog, so its
// visibility per state is the thing that can actually regress in the UI.
runTest("compact shell entry appears only while an update is actionable", () => {
  for (const status of ["available", "downloaded", "downloading", "installing"] as const) {
    assert.equal(
      shouldShowCompactUpdateEntry(makeSnapshot({ status })),
      true,
      `${status} should surface the entry`,
    );
  }

  for (const status of ["idle", "up_to_date", "checking"] as const) {
    assert.equal(
      shouldShowCompactUpdateEntry(makeSnapshot({ status })),
      false,
      `${status} should not surface the entry`,
    );
  }

  // Every error state stays out of the title bar: the dialog carries the recovery path.
  for (const errorStage of ["check", "download", "install"] as const) {
    assert.equal(
      shouldShowCompactUpdateEntry(makeSnapshot({
        status: "error",
        errorStage,
        latestVersion: "0.2.3",
        assetDownloadUrl: "https://example.com/update.exe",
      })),
      false,
      `error/${errorStage} should not surface the entry`,
    );
  }
});

runTest("confirm dialog opens for active update states and structured error states", () => {
  assert.equal(shouldOpenUpdateDialogForSnapshot(makeSnapshot({ status: "available" })), true);
  assert.equal(shouldOpenUpdateDialogForSnapshot(makeSnapshot({ status: "downloaded" })), true);
  assert.equal(shouldOpenUpdateDialogForSnapshot(makeSnapshot({ status: "downloading" })), true);
  assert.equal(shouldOpenUpdateDialogForSnapshot(makeSnapshot({ status: "installing" })), true);
  assert.equal(shouldOpenUpdateDialogForSnapshot(makeSnapshot({
    status: "error",
    errorStage: "download",
    latestVersion: "0.2.0",
  })), true);
  assert.equal(shouldOpenUpdateDialogForSnapshot(makeSnapshot({
    status: "error",
    errorStage: null,
    releasePageUrl: null,
    assetDownloadUrl: null,
    latestVersion: null,
  })), false);
});

runTest("confirm dialog model includes notes preview", () => {
  const model = buildUpdateConfirmDialogModel(makeSnapshot({
    status: "available",
    latestVersion: "0.2.0",
    releaseNotes: "A".repeat(260),
  }));
  assert.equal(model.title, "发现新版本");
  assert.equal(model.primaryAction?.label, "立即下载");
  assert.equal(model.versionCompareLabel, "v0.1.0 -> v0.2.0");
  assert.ok(model.notesPreview !== null);
  assert.ok(model.notesPreview!.length <= 223);
});

runTest("confirm dialog localizes structured release notes", () => {
  const snapshot = makeSnapshot({
    status: "available",
    latestVersion: "0.2.0",
    releaseNotes: "zh-CN: 改进应用映射与备份恢复。\nen-US: Improved app mapping and backup restore.",
  });
  const model = buildUpdateConfirmDialogModelRaw(snapshot, EN_TEXT, "en-US");

  assert.equal(model.notesPreview, "Improved app mapping and backup restore.");
  for (const locale of ["ru-RU", "es"] as const) {
    const fallbackModel = buildUpdateConfirmDialogModelRaw(snapshot, EN_TEXT, locale);
    assert.equal(fallbackModel.notesPreview, "Improved app mapping and backup restore.");
  }
});

runTest("confirm dialog shows progress while downloading", () => {
  const model = buildUpdateConfirmDialogModel(makeSnapshot({
    status: "downloading",
    latestVersion: "0.2.0",
    downloadedBytes: 768,
    totalBytes: 1024,
  }));

  assert.equal(model.title, "正在下载更新");
  assert.equal(model.progress?.valueText, "75%");
  assert.equal(model.progress?.indeterminate, false);
});

runTest("confirm dialog shows manual fallback actions for download errors", () => {
  const model = buildUpdateConfirmDialogModel(makeSnapshot({
    status: "error",
    errorStage: "download",
    errorMessage: "failed to download update",
    latestVersion: "0.2.3",
    assetDownloadUrl: "https://example.com/update.exe",
  }));

  assert.equal(model.title, "下载更新失败");
  assert.equal(model.confirmDescription.includes("failed to download update"), false);
  assert.equal(model.primaryAction?.action, "open_download_url");
  assert.equal(model.secondaryAction?.action, "check");
});

runTest("update relaunch view storage persists current keys", () => {
  withWindowStorage(new MemoryStorage(), () => {
    rememberLastActiveView("settings");
    assert.equal(readLastActiveView(), "settings");

    markPendingUpdateRelaunchViewRestore();
    assert.equal(consumePendingUpdateRelaunchView(), "settings");
    assert.equal(window.localStorage.getItem("patina:last-active-view"), "settings");
    assert.equal(window.localStorage.getItem("patina:pending-update-relaunch-view"), null);

    markPendingUpdateRelaunchViewRestore();
    assert.equal(window.localStorage.getItem("patina:pending-update-relaunch-view"), "1");

    clearPendingUpdateRelaunchViewRestore();
    assert.equal(window.localStorage.getItem("patina:pending-update-relaunch-view"), null);
  });
});

console.log(`Passed ${passed} update view model tests`);
