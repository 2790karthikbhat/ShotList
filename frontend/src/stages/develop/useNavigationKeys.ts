import { useEffect, useCallback } from "react";
import { useStore } from "../../store";

export function useNavigationKeys() {
  const navigate = useStore((s) => s.navigate);
  const session = useStore((s) => s.session);

  const handleKey = useCallback(
    (e: KeyboardEvent) => {
      if (!session) return;
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;
      switch (e.key) {
        case "ArrowRight":
        case "l":
        case "L":
          e.preventDefault();
          navigate("next");
          break;
        case "ArrowLeft":
        case "j":
        case "J":
          e.preventDefault();
          navigate("prev");
          break;
      }
    },
    [session, navigate],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [handleKey]);
}
