import { useEffect, useState } from "react";
import { Script } from "../models/scripts-api";
import { ajax } from "rxjs/ajax";

export const useGetScript = (baseURL: string, name: string) => {
  const [script, setScript] = useState<Script>();
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState();

  useEffect(() => {
    setErr(undefined);
    setLoading(true);
    const s = ajax({
      url: `${baseURL}/scripts/read`,
      method: `POST`,
      body: {
        name: name,
      },
      timeout: 2000,
    }).subscribe({
      next: (e) => {
        setScript(e.response as Script);
        setLoading(false);
      },
      error: (err) => {
        setErr(err);
        setLoading(false);
      },
    });
    return () => s.unsubscribe();
  }, [baseURL, name]);

  return { script, loading, err };
};
