import { useState } from "react";
import { Modal } from "./Modal";

interface Props {
  title: string;
  subtitle?: string;
  label: string;
  placeholder?: string;
  secondaryLabel?: string;
  secondaryPlaceholder?: string;
  submitLabel?: string;
  danger?: boolean;
  onClose: () => void;
  onSubmit: (value: string, secondary: string) => Promise<void> | void;
}

export function PromptModal({
  title,
  subtitle,
  label,
  placeholder,
  secondaryLabel,
  secondaryPlaceholder,
  submitLabel = "Create",
  danger,
  onClose,
  onSubmit,
}: Props) {
  const [value, setValue] = useState("");
  const [secondary, setSecondary] = useState("");
  const [busy, setBusy] = useState(false);

  return (
    <Modal
      title={title}
      subtitle={subtitle}
      onClose={onClose}
      footer={
        <>
          <button className="btn ghost" onClick={onClose}>
            Cancel
          </button>
          <button
            className={`btn ${danger ? "danger" : "primary"}`}
            disabled={busy || !value.trim()}
            onClick={async () => {
              setBusy(true);
              try {
                await onSubmit(value.trim(), secondary.trim());
              } finally {
                setBusy(false);
              }
            }}
          >
            {submitLabel}
          </button>
        </>
      }
    >
      <label className="field">
        <span>{label}</span>
        <input autoFocus value={value} onChange={(e) => setValue(e.target.value)} placeholder={placeholder} />
      </label>
      {secondaryLabel && (
        <label className="field">
          <span>{secondaryLabel}</span>
          <input
            value={secondary}
            onChange={(e) => setSecondary(e.target.value)}
            placeholder={secondaryPlaceholder}
          />
        </label>
      )}
    </Modal>
  );
}
