import { useEffect, useState } from "react";
import { Observable } from "rxjs";

export function useObservable<T>(
  observable: Observable<T> | undefined
): T | undefined {
  const [state, setState] = useState<T>();

  useEffect(() => {
    const sub = observable?.subscribe({
      next: (s) => setState(s),
    });
    return () => sub?.unsubscribe();
  }, [observable]);

  return state;
}
