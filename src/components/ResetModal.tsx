import { useState } from "react";
import { Modal } from "./Modal";

interface Props {
  oid: string;
  branch: string | null;
  onClose: () => void;
  onConfirm: (mode: "soft" | "mixed" | "hard") => Promise<void>;
}

const OPTIONS: { mode: "soft" | "mixed" | "hard"; title: string; detail: string }[] = [
  {
    mode: "soft",
    title: "Keep changes staged",
    detail: "Moves the branch back. Your files and index stay exactly as they are, ready to recommit.",
  },
  {
    mode: "mixed",
    title: "Keep changes unstaged",
    detail: "Moves the branch back and resets the index. Your file contents are untouched.",
  },
  {
    mode: "hard",
    title: "Discard changes",
    detail: "Moves the branch back and overwrites tracked files. Uncommitted work is lost unless a snapshot exists.",
  },
];

export function ResetModal({ oid, branch, onClose, onConfirm }: Props) {
  const [mode, setMode] = useState<"soft" | "mixed" | "hard">("mixed");
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState(false);
  const canRun = mode !== "hard" || typed.trim() === "discard";

  return (
    <Modal
      title={`Reset ${branch ?? "HEAD"} to ${oid.slice(0, 7)}`}
      subtitle="A recovery snapshot is created before the reset runs."
      width={560}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button
            className={`btn ${mode === "hard" ? "danger" : "primary"}`}
            disabled={!canRun || busy}
            onClick={async () => {
              setBusy(true);
              try {
                await onConfirm(mode);
              } finally {
                setBusy(false);
              }
            }}
          >
            {mode === "hard" ? "Discard and reset" : "Reset"}
          </button>
        </>
      }
    >
      <div className="radio-list">
        {OPTIONS.map((o) => (
          <label key={o.mode} className={`radio-card ${mode === o.mode ? "selected" : ""} ${o.mode === "hard" ? "danger" : ""}`}>
            <input type="radio" name="reset-mode" checked={mode === o.mode} onChange={() => setMode(o.mode)} />
            <div>
              <div className="radio-title">{o.title}</div>
              <div className="radio-detail">{o.detail}</div>
            </div>
          </label>
        ))}
      </div>
      {mode === "hard" && (
        <label className="field">
          <span>Type “discard” to confirm</span>
          <input value={typed} onChange={(e) => setTyped(e.target.value)} autoFocus />
        </label>
      )}
    </Modal>
  );
}
