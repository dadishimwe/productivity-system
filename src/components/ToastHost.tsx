import {
  useEffect,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";

type Toast = { id: number; message: string };

let setToastsRef: Dispatch<SetStateAction<Toast[]>> | null = null;
let nextId = 0;

export function showToast(message: string, durationMs = 3500) {
  if (!setToastsRef) return;
  const id = ++nextId;
  setToastsRef((prev) => [...prev, { id, message }]);
  window.setTimeout(() => {
    setToastsRef?.((prev) => prev.filter((t) => t.id !== id));
  }, durationMs);
}

export function ToastHost() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    setToastsRef = setToasts;
    return () => {
      setToastsRef = null;
    };
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[60] flex flex-col gap-2">
      {toasts.map((t) => (
        <div
          key={t.id}
          className="pointer-events-auto rounded-lg border border-zinc-700 bg-zinc-900 px-4 py-2 text-sm text-zinc-100 shadow-lg"
          role="status"
        >
          {t.message}
        </div>
      ))}
    </div>
  );
}
