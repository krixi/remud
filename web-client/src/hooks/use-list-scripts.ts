import { useEffect, useState } from "react";
import { ajax } from "rxjs/ajax";
import { ListScriptsResp, ScriptInfo } from "../models/scripts-api";
import { useAuth } from "./use-auth";

const sortScripts = (a: ScriptInfo, b: ScriptInfo): number => {
  if (a.name < b.name) {
    return -1;
  }
  if (a.name > b.name) {
    return 1;
  }
  return 0;
};

export const useListScripts = (baseURL: string) => {
  const [scripts, setScripts] = useState<ScriptInfo[]>();
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
      url: `${baseURL}/scripts/read/all`,
      method: `POST`,
      body: {},
      timeout: 2000,
      headers: {
        Authorization: `Bearer ${user.tokens.access_token}`,
      },
    }).subscribe({
      next: (e) => {
        const r = e.response as ListScriptsResp;
        setScripts(r.scripts.sort(sortScripts));
        setLoading(false);
      },
      error: (err) => {
        setErr(err);
        setLoading(false);
      },
    });
    return () => s.unsubscribe();
  }, [baseURL, user]);

  return { scripts, loading, err };
};
