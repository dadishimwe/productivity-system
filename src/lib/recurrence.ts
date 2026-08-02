export type RecurrencePreset = "none" | "daily" | "weekly" | "monthly";

const WEEKDAY = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"] as const;

export function rruleFromPreset(
  preset: RecurrencePreset,
  anchor: Date,
): string | null {
  switch (preset) {
    case "none":
      return null;
    case "daily":
      return "FREQ=DAILY";
    case "weekly": {
      const day = WEEKDAY[anchor.getDay()];
      return `FREQ=WEEKLY;BYDAY=${day}`;
    }
    case "monthly":
      return `FREQ=MONTHLY;BYMONTHDAY=${anchor.getDate()}`;
    default:
      return null;
  }
}

export function presetFromRrule(rrule: string | null): RecurrencePreset {
  if (!rrule) return "none";
  if (rrule.includes("FREQ=DAILY")) return "daily";
  if (rrule.includes("FREQ=WEEKLY")) return "weekly";
  if (rrule.includes("FREQ=MONTHLY")) return "monthly";
  return "none";
}

export const RECURRENCE_LABELS: Record<RecurrencePreset, string> = {
  none: "Does not repeat",
  daily: "Daily",
  weekly: "Weekly",
  monthly: "Monthly",
};
