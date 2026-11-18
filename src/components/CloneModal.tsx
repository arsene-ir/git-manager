import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errorText } from "../api";
import { Modal } from "./Modal";
import type { RepoInfo } from "../types";

interface Props {
  onClose: () => void;
  onCloned: (repo: RepoInfo) => void;
  toast: (kind: "info" | "error" | "success", text: string) => void;
}

function repoNameFromUrl(url: string): string {
  const cleaned = url.replace(/\.git$/, "").replace(/\/+$/, "");
  const last = cleaned.split(/[/:]/).filter(Boolean).pop();
  return last || "repository";
}

export function CloneModal({ onClose, onCloned, toast }: Props) {
  const [url, setUrl] = useState("");
  const [parent, setParent] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const dest = parent && url ? `${parent.replace(/[\\/]+$/, "")}/${repoNameFromUrl(url)}` : "";

  async function chooseFolder() {
    const picked = await open({ directory: true, multiple: false, title: "Choose where to clone" });
    if (typeof picked === "string") setParent(picked);
  }

  async function clone() {
    if (!url.trim() || !dest) return;
    setBusy(true);
    setError(null);
    try {
      const repo = await api.cloneRepo(url.trim(), dest);
      toast("success", `Cloned ${repo.name}`);
      onCloned(repo);
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal
      title="Clone repository"
      subtitle="Cloned through system git, so your existing credentials, SSH agent and LFS setup apply."
      width={560}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="btn primary" disabled={busy || !dest} onClick={() => void clone()}>
            {busy ? "Cloning…" : "Clone"}
          </button>
        </>
      }
    >
      <label className="field">
        <span>Remote URL</span>
        <input
          autoFocus
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://github.com/owner/repo.git or git@github.com:owner/repo.git"
        />
      </label>
      <label className="field">
        <span>Destination folder</span>
        <div className="input-row">
          <input value={parent} onChange={(e) => setParent(e.target.value)} placeholder="Choose a folder…" />
          <button className="btn ghost" onClick={() => void chooseFolder()}>
            Browse
          </button>
        </div>
      </label>
      {dest && <div className="route-card"><div><div className="route-label">Will clone into</div><div className="route-value small">{dest}</div></div></div>}
      {error && <div className="inline-error">{error}</div>}
    </Modal>
  );
}
