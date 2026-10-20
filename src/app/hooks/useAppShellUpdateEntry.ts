import { useUpdateDialog } from "../providers/UpdateDialogProvider";

interface TitleBarUpdateEntry {
  showUpdateEntry: boolean;
  onOpenUpdateDialog: () => void;
}

export function useAppShellUpdateEntry() {
  const { shouldShowCompactEntry, openUpdateDialog } = useUpdateDialog();

  const titleBarUpdateEntry: TitleBarUpdateEntry = {
    showUpdateEntry: shouldShowCompactEntry,
    onOpenUpdateDialog: openUpdateDialog,
  };

  return { titleBarUpdateEntry };
}
