import { useEffect, useState } from "react";
import { ajax } from "rxjs/ajax";
import { useAuth } from "./use-auth";
import { Script } from "../models/scripts-api";

export const useGetScript = (baseURL: string, name: string) => {
  const [script, setScript] = useState<Script>();
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<Error>();
  const { user } = useAuth();

  useEffect(() => {
    if (!user || !user.tokens) {
      setErr(new Error("logged in user required"));
      return;
    }
    setErr(undefined);
    setLoading(true);
    const s = ajax({
      url: `${baseURL}/scripts/read`,
      method: `POST`,
      body: {
        name: name,
      },
      timeout: 2000,
      headers: {
        Authorization: `Bearer ${user.tokens.access_token}`,
      },
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
  }, [baseURL, name, user]);

  return { script, loading, err };
};
