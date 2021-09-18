import { useEffect, useState } from "react";
import { ajax, AjaxError } from "rxjs/ajax";
import { ListScriptsResp, ScriptInfo } from "../models/scripts-api";

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
  const [err, setErr] = useState<AjaxError>();

  useEffect(() => {
    setErr(undefined);
    setLoading(true);
    const s = ajax({
      url: `${baseURL}/scripts/read/all`,
      method: `POST`,
      body: {},
      timeout: 2000,
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
  }, [baseURL]);

  return { scripts, loading, err };
};
