import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { useLabels } from "../labels";

const isMac = navigator.platform.toLowerCase().includes("mac");

/** The keyboard shortcuts, from the labels table; opened with mod+/ or from Settings › About. */
export function ShortcutsDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const words = useLabels().desktop.shortcuts;
  const mod = isMac ? words.mod_mac : words.mod_other;
  useEffect(() => {
    if (open && !dialog.current?.open) dialog.current?.showModal();
    if (!open && dialog.current?.open) dialog.current?.close();
  }, [open]);
  const rows: Array<[string, string[]]> = [
    [words.search, [mod, "K"]],
    [words.help, [mod, "/"]],
    [words.zoom_in, [mod, "+"]],
    [words.zoom_out, [mod, "−"]],
    [words.zoom_reset, [mod, "0"]],
    [words.escape, ["Esc"]],
  ];
  return (
    <dialog ref={dialog} className="collection-dialog shortcuts-dialog" aria-labelledby="shortcuts-title" onClose={onClose}>
      <header>
        <h2 id="shortcuts-title">{words.title}</h2>
        <button className="quiet-button" aria-label={words.close} onClick={() => dialog.current?.close()}>
          <X size={18} />
        </button>
      </header>
      <dl className="shortcut-list">
        {rows.map(([description, keys]) => (
          <div key={description}>
            <dt>{description}</dt>
            <dd>
              {keys.map((key) => (
                <kbd key={key}>{key}</kbd>
              ))}
            </dd>
          </div>
        ))}
      </dl>
    </dialog>
  );
}
