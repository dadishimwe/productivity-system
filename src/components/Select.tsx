import { useEffect, useRef, useState } from "react";

export type SelectOption = { value: string; label: string };

export function Select({
  value,
  options,
  onChange,
  placeholder = "Select…",
  className = "",
  "aria-label": ariaLabel,
}: {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  "aria-label"?: string;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  const selected = options.find((o) => o.value === value);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  return (
    <div ref={rootRef} className={`relative ${className}`}>
      <button
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        className="flex w-full items-center justify-between gap-2 rounded-md border border-zinc-700 bg-zinc-900 px-2.5 py-1.5 text-left text-sm text-zinc-100 hover:border-zinc-600"
        onClick={() => setOpen((v) => !v)}
      >
        <span className={selected ? "" : "text-zinc-500"}>
          {selected?.label ?? placeholder}
        </span>
        <span className="text-zinc-500">▾</span>
      </button>
      {open && (
        <ul
          className="absolute z-40 mt-1 max-h-56 w-full overflow-auto rounded-md border border-zinc-700 bg-zinc-900 py-1 shadow-lg"
          role="listbox"
        >
          {options.length === 0 && (
            <li className="px-3 py-2 text-sm text-zinc-500">{placeholder}</li>
          )}
          {options.map((opt) => (
            <li key={opt.value} role="option" aria-selected={opt.value === value}>
              <button
                type="button"
                className={`block w-full px-3 py-1.5 text-left text-sm hover:bg-zinc-800 ${
                  opt.value === value
                    ? "bg-zinc-800/80 text-zinc-100"
                    : "text-zinc-300"
                }`}
                onClick={() => {
                  onChange(opt.value);
                  setOpen(false);
                }}
              >
                {opt.label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
