import { useState } from "react";
import { IconChevronDown } from "../../components/icons";

export function Section({
  title,
  children,
  defaultOpen = true,
}: {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="bg-surface rounded-xl overflow-hidden flex-shrink-0">
      <button
        onClick={() => setOpen((o) => !o)}
        className="w-full flex items-center justify-between px-3.5 py-2.5 text-left hover:bg-white/5 transition-colors"
      >
        <span className="text-[11px] uppercase tracking-widest text-mid font-medium">
          {title}
        </span>
        <IconChevronDown className={`text-lo text-lg transition-transform duration-150 ${open ? "rotate-180" : ""}`} />
      </button>
      {open && <div className="px-3.5 pb-4 space-y-4">{children}</div>}
    </div>
  );
}
